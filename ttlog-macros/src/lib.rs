use proc_macro::TokenStream;
use quote::quote;
use syn::{
  parse::{Parse, ParseStream},
  parse_macro_input, Expr, Ident, LitStr, Token,
};

struct LogInput {
  kvs: Vec<(Ident, Expr)>,
  message: Option<LitStr>,
}

impl Parse for LogInput {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let mut kvs = Vec::new();
    let mut message = None;

    while !input.is_empty() {
      if input.peek(LitStr) {
        if message.is_some() {
          return Err(input.error("multiple message strings not allowed"));
        }
        message = Some(input.parse()?);
      } else {
        let key: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let value: Expr = input.parse()?;
        kvs.push((key, value));
      }

      if input.peek(Token![,]) {
        input.parse::<Token![,]>()?;
      }
    }

    Ok(LogInput { kvs, message })
  }
}

fn generate_log_call(level: u8, parsed: LogInput) -> TokenStream {
  let thread_id_expr = quote! {
    {
      static CACHED_THREAD_ID: std::sync::OnceLock<u8> = std::sync::OnceLock::new();
      *CACHED_THREAD_ID.get_or_init(|| {
        ttlog::utils::current_thread_id_u32() as u8
      })
    }
  };

  let common_constants = quote! {
    const LEVEL: u8 = #level;
    const MODULE: &'static str = module_path!();
    const FILE: &'static str = file!();
    const POSITION: (u32, u32) = (line!(), column!());
  };

  let common_statics = quote! {
    static TARGET_ID: std::sync::OnceLock<u16> = std::sync::OnceLock::new();
    static FILE_ID: std::sync::OnceLock<u16> = std::sync::OnceLock::new();
  };

  let common_kv_code = quote! {
    struct SmallVec(smallvec::SmallVec<[u8; 128]>);

    impl SmallVec {
      pub fn with_capacity(cap: usize) -> Self {
        SmallVec(smallvec::SmallVec::with_capacity(cap))
      }
    }

    impl std::io::Write for SmallVec {
      fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
      }

      fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
      }
    }


    struct IntOrSer<'a, T>(&'a T);
    
    impl<'a, T> serde::Serialize for IntOrSer<'a, T>
    where
      T: serde::Serialize + 'static,
    {
      fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
      where
        S: serde::Serializer,
      {

        // handle i64
        if let Some(i) = (self.0 as &dyn std::any::Any).downcast_ref::<i64>() {
          let mut buf = itoa::Buffer::new();
          return serializer.serialize_str(buf.format(*i));
        }
    
        // handle u64
        if let Some(u) = (self.0 as &dyn std::any::Any).downcast_ref::<u64>() {
          let mut buf = itoa::Buffer::new();
          return serializer.serialize_str(buf.format(*u));
        }

        // handle f64
        if let Some(f) = (self.0 as &dyn std::any::Any).downcast_ref::<f64>() {
          let mut buf = ryu::Buffer::new();
          return serializer.serialize_str(buf.format(*f));
        }

        // handle f32
        if let Some(f) = (self.0 as &dyn std::any::Any).downcast_ref::<f32>() {
          let mut buf = ryu::Buffer::new();
          return serializer.serialize_str(buf.format(*f));
        }
    
        self.0.serialize(serializer)
      }
    }
  };

  match (parsed.message, parsed.kvs.is_empty()) {
    // Case 1: Simple message, no key-values - FASTEST PATH
    (Some(message), true) => {
      quote! {
        {
          #common_constants
          const MESSAGE: &'static str = #message;

          #common_statics
          static MESSAGE_ID: std::sync::OnceLock<u16> = std::sync::OnceLock::new();

          ttlog::trace::GLOBAL_LOGGER.with(|logger_cell| {
            if let Some(logger) = logger_cell.get() {
              if LEVEL >= logger.level.load(std::sync::atomic::Ordering::Relaxed) {
                let target_id = *TARGET_ID.get_or_init(|| logger.interner.intern_target(MODULE));
                let message_id = *MESSAGE_ID.get_or_init(|| logger.interner.intern_message(MESSAGE));
                let file_id = *FILE_ID.get_or_init(|| logger.interner.intern_file(FILE));

                // FIX: Always create NonZeroU16 from the interned ID
                logger.send_event_fast(
                  LEVEL, 
                  target_id, 
                  std::num::NonZeroU16::new(message_id), 
                  #thread_id_expr, 
                  file_id, 
                  POSITION, 
                  None
                );
              }
            }
          });
        }
      }
    },

    // Case 2: Message with key-values
    (Some(message), false) => {
      let kv_keys: Vec<_> = parsed.kvs.iter().map(|(k, _)| k).collect();
      let kv_values = parsed.kvs.iter().map(|(_, v)| v);
      let num_kvs = parsed.kvs.len();

      quote! {
        {
          #common_constants
          const MESSAGE: &'static str = #message;
          const NUM_VALUES: usize = #num_kvs;

          #common_statics
          static MESSAGE_ID: std::sync::OnceLock<u16> = std::sync::OnceLock::new();
          static KV_ID: std::sync::OnceLock<u16> = std::sync::OnceLock::new();

          ttlog::trace::GLOBAL_LOGGER.with(|logger_cell| {
            if let Some(logger) = logger_cell.get() {
              if LEVEL >= logger.level.load(std::sync::atomic::Ordering::Relaxed) {
                
                #common_kv_code
                let mut buf = SmallVec::with_capacity(128);
                {
                  use serde::ser::{SerializeMap, Serializer};
                  let mut ser = serde_json::Serializer::new(&mut buf);
                  let mut map = match ser.serialize_map(Some(NUM_VALUES)) {
                    Ok(m) => m,
                    Err(_) => return,
                  };
                  #({
                    let wrapper = IntOrSer(&#kv_values);
                    map.serialize_entry(stringify!(#kv_keys), &wrapper).unwrap();
                  })*
                  map.end().unwrap();
                }
                
                let target_id = *TARGET_ID.get_or_init(|| logger.interner.intern_target(MODULE));
                let file_id = *FILE_ID.get_or_init(|| logger.interner.intern_file(FILE));
                let message_id = *MESSAGE_ID.get_or_init(|| logger.interner.intern_message(MESSAGE));
                let kv_id = *KV_ID.get_or_init(|| logger.interner.intern_kv(buf.0));

                logger.send_event_fast(
                  LEVEL,
                  target_id,
                  std::num::NonZeroU16::new(message_id),
                  #thread_id_expr,
                  file_id,
                  POSITION,
                  std::num::NonZeroU16::new(kv_id),
                );
              }
            }
          });
        }
      }
    },

    // Case 3: No message, only key-values
    (None, false) => {
      let kv_keys: Vec<_> = parsed.kvs.iter().map(|(_k, _)| stringify!(k)).collect();
      let kv_values = parsed.kvs.iter().map(|(_, v)| v);
      let num_kvs = parsed.kvs.len();

      quote! {
        {
          #common_constants
          const NUM_VALUES: usize = #num_kvs;

          #common_statics
          static KV_ID: std::sync::OnceLock<u16> = std::sync::OnceLock::new();

          ttlog::trace::GLOBAL_LOGGER.with(|logger_cell| {
            if let Some(logger) = logger_cell.get() {
              if LEVEL >= logger.level.load(std::sync::atomic::Ordering::Relaxed) {
                
                #common_kv_code
                let mut buf = SmallVec::with_capacity(128);
                {
                  use serde::ser::{SerializeMap, Serializer};
                  let mut ser = serde_json::Serializer::new(&mut buf);
                  let mut map = match ser.serialize_map(Some(NUM_VALUES)) {
                    Ok(m) => m,
                    Err(_) => return,
                  };
                  #({
                    let wrapper = IntOrSer(&#kv_values);
                    map.serialize_entry(#kv_keys, &wrapper).unwrap();
                  })*
                  map.end().unwrap();
                }
                
                let kv_id = *KV_ID.get_or_init(|| logger.interner.intern_kv(buf.0));
                let target_id = *TARGET_ID.get_or_init(|| logger.interner.intern_target(MODULE));
                let file_id = *FILE_ID.get_or_init(|| logger.interner.intern_file(FILE));

                logger.send_event_fast(
                  LEVEL, 
                  target_id, 
                  None,  // No message
                  #thread_id_expr, 
                  file_id, 
                  POSITION, 
                  std::num::NonZeroU16::new(kv_id)
                );
              }
            }
          });
        }
      }
    },

    // Case 4: Empty call - FIX: Don't use empty string
    (None, true) => {
      quote! {
        {
          #common_constants

          #common_statics

          ttlog::trace::GLOBAL_LOGGER.with(|logger_cell| {
            if let Some(logger) = logger_cell.get() {
              if LEVEL >= logger.level.load(std::sync::atomic::Ordering::Relaxed) {
                let target_id = *TARGET_ID.get_or_init(|| logger.interner.intern_target(MODULE));
                let file_id = *FILE_ID.get_or_init(|| logger.interner.intern_file(FILE));

                // FIX: Don't pass message_id for empty calls
                logger.send_event_fast(
                  LEVEL, 
                  target_id, 
                  None,  // No message
                  #thread_id_expr, 
                  file_id, 
                  POSITION, 
                  None
                );
              }
            }
          });
        }
      }
    },
  }
  .into()
}

#[proc_macro]
pub fn trace(input: TokenStream) -> TokenStream {
  let parsed = parse_macro_input!(input as LogInput);
  generate_log_call(0, parsed) // TRACE = 0
}

#[proc_macro]
pub fn debug(input: TokenStream) -> TokenStream {
  let parsed = parse_macro_input!(input as LogInput);
  generate_log_call(1, parsed) // DEBUG = 1
}

#[proc_macro]
pub fn info(input: TokenStream) -> TokenStream {
  let parsed = parse_macro_input!(input as LogInput);
  generate_log_call(2, parsed) // INFO = 2
}

#[proc_macro]
pub fn warn(input: TokenStream) -> TokenStream {
  let parsed = parse_macro_input!(input as LogInput);
  generate_log_call(3, parsed) // WARN = 3
}

#[proc_macro]
pub fn error(input: TokenStream) -> TokenStream {
  let parsed = parse_macro_input!(input as LogInput);
  generate_log_call(4, parsed) // ERROR = 4
}

#[proc_macro]
pub fn fatal(input: TokenStream) -> TokenStream {
  let parsed = parse_macro_input!(input as LogInput);
  generate_log_call(5, parsed) // FATAL = 5
}


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
    ttlog::utils::current_thread_id_u32() as u8
  };

  let common_constants = quote! {
    const LEVEL: u8 = #level;
    const MODULE: &str = module_path!();
    const FILE: &str = file!();
    const POSITION: (u32, u32) = (line!(), column!());
  };

  let common_statics = quote! {
    static TARGET_ID: std::sync::OnceLock<u16> = std::sync::OnceLock::new();
    static FILE_ID: std::sync::OnceLock<u16> = std::sync::OnceLock::new();
  };

  // Different expansion paths
  match (parsed.message, parsed.kvs.is_empty()) {
    // Case 1: Message only
    (Some(message), true) => quote! {
      {
        #common_constants
        const MESSAGE: &str = #message;

        #common_statics
        static MESSAGE_ID: std::sync::OnceLock<u16> = std::sync::OnceLock::new();

        if let Some(logger) = ttlog::trace::GLOBAL_LOGGER.get() {
          if LEVEL >= logger.level.load(std::sync::atomic::Ordering::Relaxed) {
            let target_id = *TARGET_ID.get_or_init(|| logger.interner.intern_target(MODULE));
            let message_id = *MESSAGE_ID.get_or_init(|| logger.interner.intern_message(MESSAGE));
            let file_id = *FILE_ID.get_or_init(|| logger.interner.intern_file(FILE));

            logger.send_event_fast(
              LEVEL,
              target_id,
              std::num::NonZeroU16::new(message_id),
              #thread_id_expr,
              file_id,
              POSITION,
              None,
            );
          }
        }
      }
    },

    // Case 2: Message + KV
    (Some(message), false) => {
      let kv_keys: Vec<_> = parsed.kvs.iter().map(|(k, _)| k).collect();
      let kv_values = parsed.kvs.iter().map(|(_, v)| v);
      let num_kvs = parsed.kvs.len();

      quote! {
        {
          #common_constants
          const MESSAGE: &str = #message;
          const NUM_VALUES: usize = #num_kvs;

          #common_statics
          static MESSAGE_ID: std::sync::OnceLock<u16> = std::sync::OnceLock::new();

          if let Some(logger) = ttlog::trace::GLOBAL_LOGGER.get() {
            if LEVEL >= logger.level.load(std::sync::atomic::Ordering::Relaxed) {
              let mut buf = ttlog::kv::KvTransformer::with_capacity(128);
              {
                use std::io::Write;
                // SEC-022: serialize each field independently so a single
                // failing `Serialize` impl drops ONLY that field — every
                // other kv pair survives. The bad field gets a placeholder.
                let mut __kv_map = serde_json::Map::with_capacity(NUM_VALUES);
                #({
                  let __kv_val = serde_json::to_value(
                    ttlog::kv::IntOrSer(&#kv_values)
                  ).unwrap_or_else(|_| {
                    serde_json::Value::String("<serialize-error>".to_string())
                  });
                  __kv_map.insert(stringify!(#kv_keys).to_string(), __kv_val);
                })*
                let _ = serde_json::to_writer(&mut buf, &__kv_map)
                  .or_else(|_| buf.write_all(b"{}"));
              }

              let target_id = *TARGET_ID.get_or_init(|| logger.interner.intern_target(MODULE));
              let file_id = *FILE_ID.get_or_init(|| logger.interner.intern_file(FILE));
              let message_id = *MESSAGE_ID.get_or_init(|| logger.interner.intern_message(MESSAGE));
              let kv_id = logger.interner.intern_kv(buf.into_inner());

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
        }
      }
    },

    // Case 3: KV only
    (None, false) => {
      let kv_keys: Vec<_> = parsed.kvs.iter().map(|(k, _)| k).collect();
      let kv_values = parsed.kvs.iter().map(|(_, v)| v);
      let num_kvs = parsed.kvs.len();

      quote! {
        {
          #common_constants
          const NUM_VALUES: usize = #num_kvs;

          #common_statics

          if let Some(logger) = ttlog::trace::GLOBAL_LOGGER.get() {
            if LEVEL >= logger.level.load(std::sync::atomic::Ordering::Relaxed) {
              let mut buf = ttlog::kv::KvTransformer::with_capacity(128);
              {
                use std::io::Write;
                // SEC-022: serialize each field independently so a single
                // failing `Serialize` impl drops ONLY that field — every
                // other kv pair survives. The bad field gets a placeholder.
                let mut __kv_map = serde_json::Map::with_capacity(NUM_VALUES);
                #({
                  let __kv_val = serde_json::to_value(
                    ttlog::kv::IntOrSer(&#kv_values)
                  ).unwrap_or_else(|_| {
                    serde_json::Value::String("<serialize-error>".to_string())
                  });
                  __kv_map.insert(stringify!(#kv_keys).to_string(), __kv_val);
                })*
                let _ = serde_json::to_writer(&mut buf, &__kv_map)
                  .or_else(|_| buf.write_all(b"{}"));
              }

              let kv_id = logger.interner.intern_kv(buf.into_inner());
              let target_id = *TARGET_ID.get_or_init(|| logger.interner.intern_target(MODULE));
              let file_id = *FILE_ID.get_or_init(|| logger.interner.intern_file(FILE));

              logger.send_event_fast(
                LEVEL,
                target_id,
                None,
                #thread_id_expr,
                file_id,
                POSITION,
                std::num::NonZeroU16::new(kv_id),
              );
            }
          }
        }
      }
    },

    // Case 4: Empty call
    (None, true) => quote! {
      {
        #common_constants
        #common_statics

        if let Some(logger) = ttlog::trace::GLOBAL_LOGGER.get() {
          if LEVEL >= logger.level.load(std::sync::atomic::Ordering::Relaxed) {
            let target_id = *TARGET_ID.get_or_init(|| logger.interner.intern_target(MODULE));
            let file_id = *FILE_ID.get_or_init(|| logger.interner.intern_file(FILE));

            logger.send_event_fast(
              LEVEL,
              target_id,
              None,
              #thread_id_expr,
              file_id,
              POSITION,
              None,
            );
          }
        }
      }
    },
  }
  .into()
}

#[proc_macro]
pub fn trace(input: TokenStream) -> TokenStream {
  generate_log_call(0, parse_macro_input!(input as LogInput))
}
#[proc_macro]
pub fn debug(input: TokenStream) -> TokenStream {
  generate_log_call(1, parse_macro_input!(input as LogInput))
}
#[proc_macro]
pub fn info(input: TokenStream) -> TokenStream {
  generate_log_call(2, parse_macro_input!(input as LogInput))
}
#[proc_macro]
pub fn warn(input: TokenStream) -> TokenStream {
  generate_log_call(3, parse_macro_input!(input as LogInput))
}
#[proc_macro]
pub fn error(input: TokenStream) -> TokenStream {
  generate_log_call(4, parse_macro_input!(input as LogInput))
}
#[proc_macro]
pub fn fatal(input: TokenStream) -> TokenStream {
  generate_log_call(5, parse_macro_input!(input as LogInput))
}

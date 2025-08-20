use proc_macro::TokenStream;
use quote::quote;
use syn::{
  parse::{Parse, ParseStream},
  parse_macro_input, Expr, Ident, LitStr, Token,
};

#[derive(Debug)]
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
  // Get thread ID at compile time where possible
  let thread_id_expr = quote! {
    {
      static CACHED_THREAD_ID: std::sync::OnceLock<u8> = std::sync::OnceLock::new();
      *CACHED_THREAD_ID.get_or_init(|| {
        ttlog::event_builder::EventBuilder::current_thread_id_u32() as u8
      })
    }
  };

  match (parsed.message, parsed.kvs.is_empty()) {
    // Case 1: Simple message, no key-values - FASTEST PATH
    (Some(message), true) => {
      quote! {
        {
          const LEVEL: u8 = #level;
          const MESSAGE: &'static str = #message;
          const MODULE: &'static str = module_path!();

          // Static caching - these are computed only once per call site
          static TARGET_ID: std::sync::OnceLock<u16> = std::sync::OnceLock::new();
          static MESSAGE_ID: std::sync::OnceLock<u16> = std::sync::OnceLock::new();

          ttlog::trace::GLOBAL_LOGGER.with(|logger_cell| {
            if let Some(logger) = logger_cell.get() {
              // Ultra-fast level check
              if LEVEL >= logger.level.load(std::sync::atomic::Ordering::Relaxed) {
                let target_id = *TARGET_ID.get_or_init(||
                  logger.interner.intern_target(MODULE));
                let message_id = *MESSAGE_ID.get_or_init(||
                  logger.interner.intern_message(MESSAGE));

                logger.send_event_fast(LEVEL, target_id, message_id, #thread_id_expr);
              }
            }
          });
        }
      }
    },

    // Case 2: Message with key-values - OPTIMIZED PATH
    (Some(message), false) => {
      // Pre-build format string at compile time
      let format_parts: Vec<String> = parsed
        .kvs
        .iter()
        .map(|(k, _)| format!("{}={{:?}}", k))
        .collect();
      let format_str = if format_parts.is_empty() {
        message.value().to_string()
      } else {
        format!("{} {}", message.value(), format_parts.join(" "))
      };

      let kv_values = parsed.kvs.iter().map(|(_, v)| v);

      quote! {
        {
          const LEVEL: u8 = #level;
          const MODULE: &'static str = module_path!();

          static TARGET_ID: std::sync::OnceLock<u16> = std::sync::OnceLock::new();

          ttlog::trace::GLOBAL_LOGGER.with(|logger_cell| {
            if let Some(logger) = logger_cell.get() {
              if LEVEL >= logger.level.load(std::sync::atomic::Ordering::Relaxed) {
                let target_id = *TARGET_ID.get_or_init(||
                  logger.interner.intern_target(MODULE));

                // Format only when needed, intern immediately
                let formatted = format!(#format_str, #(#kv_values),*);
                let message_id = logger.interner.intern_message(&formatted);

                logger.send_event_fast(LEVEL, target_id, message_id, #thread_id_expr);
              }
            }
          });
        }
      }
    },

    // Case 3: No message, only key-values - COMPACT PATH
    (None, false) => {
      let format_parts: Vec<String> = parsed
        .kvs
        .iter()
        .map(|(k, _)| format!("{}={{:?}}", k))
        .collect();
      let format_str = format_parts.join(" ");
      let kv_values = parsed.kvs.iter().map(|(_, v)| v);

      quote! {
        {
          const LEVEL: u8 = #level;
          const MODULE: &'static str = module_path!();

          static TARGET_ID: std::sync::OnceLock<u16> = std::sync::OnceLock::new();

          ttlog::trace::GLOBAL_LOGGER.with(|logger_cell| {
            if let Some(logger) = logger_cell.get() {
              if LEVEL >= logger.level.load(std::sync::atomic::Ordering::Relaxed) {
                let target_id = *TARGET_ID.get_or_init(||
                  logger.interner.intern_target(MODULE));

                let formatted = format!(#format_str, #(#kv_values),*);
                let message_id = logger.interner.intern_message(&formatted);

                logger.send_event_fast(LEVEL, target_id, message_id, #thread_id_expr);
              }
            }
          });
        }
      }
    },

    // Case 4: Empty call - MINIMAL PATH
    (None, true) => {
      quote! {
        {
          const LEVEL: u8 = #level;
          const MODULE: &'static str = module_path!();

          static TARGET_ID: std::sync::OnceLock<u16> = std::sync::OnceLock::new();
          static MESSAGE_ID: std::sync::OnceLock<u16> = std::sync::OnceLock::new();

          ttlog::trace::GLOBAL_LOGGER.with(|logger_cell| {
            if let Some(logger) = logger_cell.get() {
              if LEVEL >= logger.level.load(std::sync::atomic::Ordering::Relaxed) {
                let target_id = *TARGET_ID.get_or_init(||
                  logger.interner.intern_target(MODULE));
                let message_id = *MESSAGE_ID.get_or_init(||
                  logger.interner.intern_message(""));

                logger.send_event_fast(LEVEL, target_id, message_id, #thread_id_expr);
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
pub fn debug(input: TokenStream) -> TokenStream {
  let parsed = parse_macro_input!(input as LogInput);
  generate_log_call(1, parsed) // DEBUG = 1
}

#[proc_macro]
pub fn trace(input: TokenStream) -> TokenStream {
  let parsed = parse_macro_input!(input as LogInput);
  generate_log_call(0, parsed) // TRACE = 0
}

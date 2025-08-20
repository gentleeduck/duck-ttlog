use proc_macro::TokenStream;
use quote::quote;
use syn::{
  parse::{Parse, ParseStream},
  parse_macro_input, Expr, Ident, LitStr, Token,
};
use ttlog::trace::GLOBAL_LOGGER;

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
        // found "string"
        if message.is_some() {
          return Err(input.error("multiple message strings not allowed"));
        }
        message = Some(input.parse()?);
      } else {
        // parse `key = value`
        let key: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let value: Expr = input.parse()?;
        kvs.push((key, value));
      }

      // optional comma
      if input.peek(Token![,]) {
        input.parse::<Token![,]>()?;
      }
    }

    Ok(LogInput { kvs, message })
  }
}

fn generate_log_call(level: ttlog::event::LogLevel, parsed: LogInput) -> TokenStream {
  let level_ident = level as u8;
  // want to get the stack trace for the warnings for example or errors
  // let stack_strace = std::backtrace::Backtrace::

  match (parsed.message, parsed.kvs.is_empty()) {
    (Some(message), true) => {
      quote! {
        ttlog::trace::GLOBAL_LOGGER.with(|logger_cell| {
          if let Some(logger) = logger_cell.get() {
            let current_level = logger.get_level();
            if #level_ident >= current_level as u8 {
              logger.send_event(#level_ident, #message, module_path!(), file!(), (line!(), column!()));
            }
          } else {
            eprintln!("[{}] Logger not initialized: {}", #level_ident, #message);
          }
        });
      }
    },
    (Some(message), false) => {
      let kvs = parsed
        .kvs
        .iter()
        .map(|(k, v)| {
          let k_lit = syn::LitStr::new(&k.to_string(), proc_macro2::Span::call_site());
          quote! {
            format!("{} = {:?}", #k_lit, #v)
          }
        })
        .collect::<Vec<_>>();

      quote! {
        ttlog::trace::GLOBAL_LOGGER.with(|logger_cell| {
          if let Some(logger) = logger_cell.get() {
            let current_level = logger.get_level();
            if #level_ident >= current_level as u8 {
              logger.send_event(#level_ident, &format!("{} {}", #message, [#(#kvs),*].join(" ")), module_path!());
            }
          } else {
               eprintln!("[{}] Logger not initialized: {}", #level_ident, format!("{} {}", #message, [#(#kvs),*].join(" ")));
          }
        });
      }
    },

    (None, true) => {
      quote! {
        println!("[{}] {}", #level_ident, "No message");
      }
    },

    (None, false) => {
      let kvs = parsed
        .kvs
        .iter()
        .map(|(k, v)| {
          let k_lit = syn::LitStr::new(&k.to_string(), proc_macro2::Span::call_site());
          quote! {
            format!("{} = {:?}", #k_lit, #v)
          }
        })
        .collect::<Vec<_>>();

      quote! {
        println!("[{}] {}", #level_ident, [#(#kvs),*].join(" "));
      }
    },
  }
  .into()
}

#[proc_macro]
pub fn info(input: TokenStream) -> TokenStream {
  let parsed = parse_macro_input!(input as LogInput);
  generate_log_call(ttlog::event::LogLevel::INFO, parsed)
}

// #[proc_macro]
// pub fn warn(input: TokenStream) -> TokenStream {
//   let parsed = parse_macro_input!(input as LogInput);
//   generate_log_call("WARN", parsed)
// }
//
// #[proc_macro]
// pub fn error(input: TokenStream) -> TokenStream {
//   let parsed = parse_macro_input!(input as LogInput);
//   generate_log_call("ERROR", parsed)
// }
//
// #[proc_macro]
// pub fn debug(input: TokenStream) -> TokenStream {
//   let parsed = parse_macro_input!(input as LogInput);
//   generate_log_call("DEBUG", parsed)
// }
//
// #[proc_macro]
// pub fn trace(input: TokenStream) -> TokenStream {
//   let parsed = parse_macro_input!(input as LogInput);
//   generate_log_call("TRACE", parsed)
// }
//
// #[proc_macro]
// pub fn span(input: TokenStream) -> TokenStream {
//   let parsed = parse_macro_input!(input as LogInput);
//   generate_log_call("SPAN", parsed)
// }
//
// #[proc_macro]
// pub fn todo(input: TokenStream) -> TokenStream {
//   let parsed = parse_macro_input!(input as LogInput);
//   generate_log_call("TODO", parsed)
// }
//
// #[proc_macro]
// pub fn event(input: TokenStream) -> TokenStream {
//   let parsed = parse_macro_input!(input as LogInput);
//   generate_log_call("EVENT", parsed)
// }

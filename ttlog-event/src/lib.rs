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

fn generate_log_call(level: &str, parsed: LogInput) -> TokenStream {
  let level_ident = level.to_string();

  match (parsed.message, parsed.kvs.is_empty()) {
    (Some(message), true) => {
      quote! {
        println!("[{}] {}", #level_ident, #message);
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
        println!(
          "[{}] {} {:?}",
          #level_ident,
          #message,
          [#(#kvs),*].join(" ")

        );
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
  generate_log_call("info", parsed)
}

#[proc_macro]
pub fn warn(input: TokenStream) -> TokenStream {
  let parsed = parse_macro_input!(input as LogInput);
  generate_log_call("warn", parsed)
}

#[proc_macro]
pub fn error(input: TokenStream) -> TokenStream {
  let parsed = parse_macro_input!(input as LogInput);
  generate_log_call("error", parsed)
}

#[proc_macro]
pub fn debug(input: TokenStream) -> TokenStream {
  let parsed = parse_macro_input!(input as LogInput);
  generate_log_call("debug", parsed)
}

#[proc_macro]
pub fn trace(input: TokenStream) -> TokenStream {
  let parsed = parse_macro_input!(input as LogInput);
  generate_log_call("trace", parsed)
}

#[proc_macro]
pub fn span(input: TokenStream) -> TokenStream {
  let parsed = parse_macro_input!(input as LogInput);
  generate_log_call("span", parsed)
}

#[proc_macro]
pub fn todo(input: TokenStream) -> TokenStream {
  let parsed = parse_macro_input!(input as LogInput);
  generate_log_call("todo", parsed)
}

#[proc_macro]
pub fn event(input: TokenStream) -> TokenStream {
  let parsed = parse_macro_input!(input as LogInput);
  generate_log_call("event", parsed)
}

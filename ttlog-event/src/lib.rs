use proc_macro::TokenStream;
use quote::quote;
use syn::{
  parse::{Parse, ParseStream},
  Expr, LitStr, Result, Token,
};
use ttlog::event::LogLevel;

#[derive(Debug)]
struct LogInput {
  message: Expr,
  fields: Vec<(LitStr, Expr)>,
}

// Implement parsing for LogInput
impl Parse for LogInput {
  fn parse(input: ParseStream) -> Result<Self> {
    let message: Expr = input.parse()?;
    let mut fields = Vec::new();

    // Parse optional comma-separated key = value fields
    while input.peek(Token![,]) {
      let _comma: Token![,] = input.parse()?;
      if input.is_empty() {
        break;
      }
      let key: LitStr = input.parse()?;
      let _eq: Token![=] = input.parse()?;
      let value: Expr = input.parse()?;
      fields.push((key, value));
    }

    Ok(LogInput { message, fields })
  }
}

#[proc_macro]
pub fn info(input: TokenStream) -> TokenStream {
  let input = syn::parse_macro_input!(input as LogInput);
  let expanded = generate_log_call(&input, LogLevel::INFO);
  TokenStream::from(expanded)
}

// Optimized code generation
fn generate_log_call(input: &LogInput, level: LogLevel) -> proc_macro2::TokenStream {
  println!("Generate log call for level: {:?}", input);
  let message = &input.message;
  let target = get_module_path();

  let field_names: Vec<_> = input.fields.iter().map(|(k, _)| k).collect();
  let field_values: Vec<_> = input.fields.iter().map(|(_, v)| v).collect();
  let field_count = input.fields.len();

  let level_ident = match level {
    LogLevel::INFO => quote! { Info },
    _ => quote! {Info},
  };

  quote! {
    if ttlog::logger::is_enabled(ttlog::event::LogLevel::#level_ident, #target) {
          println!("{}: {}", #target, #message);

      // let mut event = ttlog::EventBuilder::with_capacity(#field_count);
      //
      // event
      //   .timestamp(ttlog::now_nanos())
      //   .level(ttlog::LogLevel::#level_ident)
      //   .target(#target)
      //   .message(#message);
      //
      // #(event.field(#field_names, #field_values);)*
      //
      // ttlog::logger::emit_fast(event.build());
    }
  }
}

// Helper to get module path
fn get_module_path() -> &'static str {
  module_path!()
}

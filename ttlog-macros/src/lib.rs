// in hello_macro_derive/src/lib.rs

use proc_macro::TokenStream;
use quote::quote; // for generating Rust code easily
use syn; // for parsing Rust code into an AST

#[proc_macro_derive(HelloMacro)]
pub fn hello_macro_derive(input: TokenStream) -> TokenStream {
  // Parse the input into a syntax tree
  let ast = syn::parse(input).unwrap();

  // Build the implementation
  impl_hello_macro(&ast)
}

fn impl_hello_macro(ast: &syn::DeriveInput) -> TokenStream {
  let name = &ast.ident; // the name of the type
  let r = quote! {
      impl HelloMacro for #name {
          fn hello_macro() {
              println!("Hello, Macro! My name is {}!", stringify!(#name));
          }
      }
  };
  r.into()
}

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(DevCommand)]
pub fn dev_command_derive(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    // Get the name of the struct
    let name = &input.ident;

    // Generate the implementation of DevCommand for the struct
    let expanded = quote! {
        impl DevCommand for #name {}
    };

    // Convert the generated code into a TokenStream and return it
    TokenStream::from(expanded)
}

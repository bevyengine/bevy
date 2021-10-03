use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

pub fn derive_id(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let struct_name = &ast.ident;
    let error_message = format!("The system ran out of unique `{}`s.", struct_name);
    let (_impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    TokenStream::from(quote! {
        impl #struct_name #type_generics #where_clause {
            pub fn new() -> Self {
                use std::sync::atomic::{AtomicU64, Ordering};
                static COUNTER: AtomicU64 = AtomicU64::new(0);
                COUNTER
                    .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |val| {
                        val.checked_add(1)
                    })
                    .map(Self)
                    .expect(#error_message)
            }
        }
    })
}

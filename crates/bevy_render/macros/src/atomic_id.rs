use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, Ident, Visibility};

pub struct AtomicIdStruct {
    pub vis: Visibility,
    pub ident: Ident,
}

impl Parse for AtomicIdStruct {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            vis: input.parse()?,
            ident: input.parse()?,
        })
    }
}

pub fn define_atomic_id(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as AtomicIdStruct);

    let struct_name = &ast.ident;
    let visibility = &ast.vis;
    let error_message = format!("The system ran out of unique `{}`s.", struct_name);

    TokenStream::from(quote! {

        #[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
        #visibility struct #struct_name(crate::render_resource::resource_macros::AtomicIdType);

        impl #struct_name {
            pub fn new() -> Self {
                use crate::render_resource::resource_macros::AtomicIdCounter;
                use std::sync::atomic::Ordering;

                static COUNTER: AtomicIdCounter = AtomicIdCounter::new(0);
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

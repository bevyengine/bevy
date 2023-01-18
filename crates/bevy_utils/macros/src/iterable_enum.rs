use proc_macro::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::{__private::Span, spanned::Spanned, DataEnum};

use crate::paths;

pub fn parse_iterable_enum_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);

    let span = ast.span();

    let name = &ast.ident;
    let generics = &ast.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let get_at = match ast.data {
        syn::Data::Enum(d) => get_at_impl(name, span, d),
        _ => quote_spanned! {
            span => compile_error!("`IterableEnum` can only be applied to `enum`")
        },
    };

    let iterable_enum = paths::iterable_enum_path();

    quote! {
        impl #impl_generics #iterable_enum for #name #ty_generics #where_clause {
            #get_at
        }
    }
    .into()
}

fn get_at_impl(name: impl ToTokens, span: Span, d: DataEnum) -> quote::__private::TokenStream {
    let mut arms = quote!();
    let mut index: usize = 0;

    for variant in d.variants {
        match variant.fields {
            syn::Fields::Unit => {
                let ident = variant.ident;
                arms = quote! { #arms #index => Some(#name::#ident), };
                index += 1;
            }
            _ => {
                return quote_spanned! {
                    span => compile_error!("All Fields should be Units!");
                }
                .into();
            }
        };
    }

    quote! {
        #[inline]
        fn get_at(index: usize) -> Option<Self> {
            match index {
                #arms,
                _ => None,
            }
        }
    }
    .into()
}

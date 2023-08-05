use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;

pub fn static_ref_impl(input: &syn::DeriveInput) -> TokenStream {
    match &input.data {
        syn::Data::Struct(data) => {
            if data.fields.is_empty() {
                quote! { ::std::option::Option::Some(&Self) }
            } else {
                quote! { ::std::option::Option::None }
            }
        }
        syn::Data::Enum(data) => {
            let variants: Vec<_> = data
                .variants
                .iter()
                .filter_map(|variant| {
                    if variant.fields.is_empty() {
                        let span = variant.span();
                        let variant_ident = variant.ident.clone();
                        Some(quote_spanned! { span => Self::#variant_ident => ::std::option::Option::Some(&Self::#variant_ident), })
                    } else {
                        None
                    }
                })
                .collect();
            let fallback_variant = if variants.len() < data.variants.len() {
                quote!(_ => ::std::option::Option::None,)
            } else {
                quote!()
            };
            quote! {
                match self {
                    #(#variants)*
                    #fallback_variant
                }
            }
        }
        syn::Data::Union(_) => {
            quote! { ::std::option::Option::None }
        }
    }
}

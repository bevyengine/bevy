use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned;

pub fn static_ref_impl(input: &syn::DeriveInput) -> TokenStream {
    // We cannot support generics correctly since we use static items
    if input.generics.const_params().next().is_some()
        || input.generics.type_params().next().is_some()
    {
        return quote! { ::std::option::Option::None };
    }

    let ident = input.ident.clone();
    match &input.data {
        syn::Data::Struct(data) if data.fields.is_empty() => {
            quote! {
                static SELF: #ident = #ident{};
                ::std::option::Option::Some(&SELF)
            }
        }
        syn::Data::Enum(data) => {
            let variant_data: Vec<_> = data
                .variants
                .iter()
                .filter(|variant| variant.fields.is_empty())
                .map(|variant| {
                    let span = variant.span();
                    let variant_ident = variant.ident.clone();
                    let static_ident = format_ident!("SELF_{}", variant_ident);
                    (span, variant_ident, static_ident)
                })
                .collect();

            let variant_statics = variant_data
                .iter()
                .map(|(span, variant_ident, static_ident)| {
                    quote_spanned! { *span =>
                        #[allow(non_upper_case_globals)]
                        static #static_ident: #ident = #ident::#variant_ident{};
                    }
                });

            let variants = variant_data
                .iter()
                .map(|(span, variant_ident, static_ident)| {
                    quote_spanned! { *span =>
                        Self::#variant_ident{} => ::std::option::Option::Some(&#static_ident),
                    }
                });

            quote! {
                #(#variant_statics)*
                match self {
                    #(#variants)*
                    #[allow(unreachable_patterns)]
                    _ => ::std::option::Option::None,
                }
            }
        }
        _ => {
            quote! { ::std::option::Option::None }
        }
    }
}

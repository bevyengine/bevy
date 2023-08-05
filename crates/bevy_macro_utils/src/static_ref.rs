use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;

pub fn static_ref_impl(input: &syn::DeriveInput) -> TokenStream {
    match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) if fields.named.is_empty() => {
                quote! { ::std::option::Option::Some(&Self{}) }
            }
            syn::Fields::Unnamed(fields) if fields.unnamed.is_empty() => {
                quote! { ::std::option::Option::Some(&Self()) }
            }
            syn::Fields::Unit => quote! { ::std::option::Option::Some(&Self) },
            _ => {
                quote! { ::std::option::Option::None }
            }
        },
        syn::Data::Enum(data) => {
            let variants: Vec<_> = data
                .variants
                .iter()
                .filter_map(|variant| {
                    let span = variant.span();
                    let variant_ident = variant.ident.clone();
                    match &variant.fields {
                        syn::Fields::Named(fields) if fields.named.is_empty() => {
                            Some(quote_spanned! { span => Self::#variant_ident{} => ::std::option::Option::Some(&Self::#variant_ident{}), })
                        }
                        syn::Fields::Unnamed(fields) if fields.unnamed.is_empty() => {
                            // TODO: Simplify using `Self` after rustc 1.72.0 is released.
                            let ident = input.ident.clone();
                            let (_, ty_generics, _) = input.generics.split_for_impl();
                            let turbofish = ty_generics.as_turbofish();
                            Some(quote_spanned! { span => Self::#variant_ident() => ::std::option::Option::Some(&#ident #turbofish::#variant_ident()), })
                        }
                        syn::Fields::Unit => {
                            Some(quote_spanned! { span => Self::#variant_ident => ::std::option::Option::Some(&Self::#variant_ident), })
                        }
                        _ => None
                    }
                })
                .collect();
            quote! {
                match self {
                    #(#variants)*
                    #[allow(unreachable_patterns)]
                    _ => ::std::option::Option::None,
                }
            }
        }
        syn::Data::Union(_) => {
            quote! { ::std::option::Option::None }
        }
    }
}

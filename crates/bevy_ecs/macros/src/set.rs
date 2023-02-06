use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};

pub static SYSTEM_SET_ATTRIBUTE_NAME: &str = "system_set";
pub static BASE_ATTRIBUTE_NAME: &str = "base";

/// Derive a label trait
///
/// # Args
///
/// - `input`: The [`syn::DeriveInput`] for struct that is deriving the label trait
/// - `trait_path`: The path [`syn::Path`] to the label trait
pub fn derive_set(input: syn::DeriveInput, trait_path: &syn::Path) -> TokenStream {
    let ident = input.ident;

    let mut is_base = false;
    for attr in &input.attrs {
        if !attr
            .path
            .get_ident()
            .map_or(false, |ident| ident == SYSTEM_SET_ATTRIBUTE_NAME)
        {
            continue;
        }

        attr.parse_args_with(|input: ParseStream| {
            let meta = input.parse_terminated::<syn::Meta, syn::token::Comma>(syn::Meta::parse)?;
            for meta in meta {
                let ident = meta.path().get_ident().unwrap_or_else(|| {
                    panic!(
                        "Unrecognized attribute: `{}`",
                        meta.path().to_token_stream()
                    )
                });
                if ident == BASE_ATTRIBUTE_NAME {
                    if let syn::Meta::Path(_) = meta {
                        is_base = true;
                    } else {
                        panic!(
                            "The `{BASE_ATTRIBUTE_NAME}` attribute is expected to have no value or arguments",
                        );
                    }
                } else {
                    panic!(
                        "Unrecognized attribute: `{}`",
                        meta.path().to_token_stream()
                    );
                }
            }
            Ok(())
        })
        .unwrap_or_else(|_| panic!("Invalid `{SYSTEM_SET_ATTRIBUTE_NAME}` attribute format"));
    }

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let mut where_clause = where_clause.cloned().unwrap_or_else(|| syn::WhereClause {
        where_token: Default::default(),
        predicates: Default::default(),
    });
    where_clause.predicates.push(
        syn::parse2(quote! {
            Self: 'static + Send + Sync + Clone + Eq + ::std::fmt::Debug + ::std::hash::Hash
        })
        .unwrap(),
    );

    (quote! {
        impl #impl_generics #trait_path for #ident #ty_generics #where_clause {
            fn is_system_type(&self) -> bool {
                false
            }

            fn is_base(&self) -> bool {
                #is_base
            }

            fn dyn_clone(&self) -> std::boxed::Box<dyn #trait_path> {
                std::boxed::Box::new(std::clone::Clone::clone(self))
            }
        }
    })
    .into()
}

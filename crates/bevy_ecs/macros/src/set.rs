use bevy_macro_utils::BevyManifest;
use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;

/// Derive a set trait
///
/// # Args
///
/// - `input`: The [`syn::DeriveInput`] for the struct that we want to derive the set trait for
/// - `trait_path`: The [`syn::Path`] to the set trait
pub fn derive_set(input: syn::DeriveInput, trait_path: &syn::Path) -> TokenStream {
    let bevy_utils_path = BevyManifest::default().get_path("bevy_utils");

    let ident = input.ident;

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
    let dyn_static_ref_impl = match input.data {
        syn::Data::Struct(data) => {
            if data.fields.is_empty() {
                quote! { std::option::Option::Some(&Self) }
            } else {
                quote! { std::option::Option::None }
            }
        }
        syn::Data::Enum(data) => {
            let mut use_fallback_variant = false;
            let variants: Vec<_> = data
                .variants
                .into_iter()
                .filter_map(|variant| {
                    if variant.fields.is_empty() {
                        let span = variant.span();
                        let variant_ident = variant.ident;
                        Some(quote_spanned! { span => Self::#variant_ident => std::option::Option::Some(&Self::#variant_ident), })
                    } else {
                        use_fallback_variant = true;
                        None
                    }
                })
                .collect();
            if use_fallback_variant {
                quote! {
                    match self {
                        #(#variants)*
                        _ => std::option::Option::None
                    }
                }
            } else {
                quote! {
                    match self {
                        #(#variants)*
                    }
                }
            }
        }
        syn::Data::Union(_) => quote! { std::option::Option::None },
    };
    (quote! {
        impl #impl_generics #trait_path for #ident #ty_generics #where_clause {
            fn dyn_clone(&self) -> std::boxed::Box<dyn #trait_path> {
                std::boxed::Box::new(std::clone::Clone::clone(self))
            }

            fn as_dyn_eq(&self) -> &dyn #bevy_utils_path::label::DynEq {
                self
            }

            fn dyn_hash(&self, mut state: &mut dyn ::std::hash::Hasher) {
                std::hash::Hash::hash(&std::any::TypeId::of::<Self>(), &mut state);
                std::hash::Hash::hash(self, &mut state);
            }

            fn dyn_static_ref(&self) -> std::option::Option<&'static dyn #trait_path> {
                #dyn_static_ref_impl
            }
        }
    })
    .into()
}

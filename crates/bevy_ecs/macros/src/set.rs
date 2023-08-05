use bevy_macro_utils::{static_ref_impl, BevyManifest};
use proc_macro::TokenStream;
use quote::quote;

/// Derive a set trait
///
/// # Args
///
/// - `input`: The [`syn::DeriveInput`] for the struct that we want to derive the set trait for
/// - `trait_path`: The [`syn::Path`] to the set trait
pub fn derive_set(input: syn::DeriveInput, trait_path: &syn::Path) -> TokenStream {
    let bevy_utils_path = BevyManifest::default().get_path("bevy_utils");

    let ident = input.ident.clone();

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
    let dyn_static_ref_impl = match static_ref_impl(&input) {
        Ok(stream) => stream,
        Err(stream) => return stream.into(),
    };
    (quote! {
        impl #impl_generics #trait_path for #ident #ty_generics #where_clause {
            fn dyn_clone(&self) -> ::std::boxed::Box<dyn #trait_path> {
                ::std::boxed::Box::new(::std::clone::Clone::clone(self))
            }

            fn as_dyn_eq(&self) -> &dyn #bevy_utils_path::label::DynEq {
                self
            }

            fn dyn_hash(&self, mut state: &mut dyn ::std::hash::Hasher) {
                ::std::hash::Hash::hash(&::std::any::TypeId::of::<Self>(), &mut state);
                ::std::hash::Hash::hash(self, &mut state);
            }

            fn dyn_static_ref(&self) -> ::std::option::Option<&'static dyn #trait_path> {
                #dyn_static_ref_impl
            }
        }
    })
    .into()
}

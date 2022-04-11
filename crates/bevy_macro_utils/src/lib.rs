extern crate proc_macro;

mod attrs;
mod shape;
mod symbol;

pub use attrs::*;
pub use shape::*;
pub use symbol::*;

use cargo_manifest::{DepsSet, Manifest};
use proc_macro::TokenStream;
use quote::quote;
use std::{env, path::PathBuf};

pub struct BevyManifest {
    manifest: Manifest,
}

impl Default for BevyManifest {
    fn default() -> Self {
        Self {
            manifest: env::var_os("CARGO_MANIFEST_DIR")
                .map(PathBuf::from)
                .map(|mut path| {
                    path.push("Cargo.toml");
                    Manifest::from_path(path).unwrap()
                })
                .unwrap(),
        }
    }
}

impl BevyManifest {
    pub fn maybe_get_path(&self, name: &str) -> Option<syn::Path> {
        const BEVY: &str = "bevy";
        const BEVY_INTERNAL: &str = "bevy_internal";

        let find_in_deps = |deps: &DepsSet| -> Option<syn::Path> {
            let package = if let Some(dep) = deps.get(name) {
                return Some(Self::parse_str(dep.package().unwrap_or(name)));
            } else if let Some(dep) = deps.get(BEVY) {
                dep.package().unwrap_or(BEVY)
            } else if let Some(dep) = deps.get(BEVY_INTERNAL) {
                dep.package().unwrap_or(BEVY_INTERNAL)
            } else {
                return None;
            };

            let mut path = Self::parse_str::<syn::Path>(package);
            if let Some(module) = name.strip_prefix("bevy_") {
                path.segments.push(Self::parse_str(module));
            }
            Some(path)
        };

        let deps = self.manifest.dependencies.as_ref();
        let deps_dev = self.manifest.dev_dependencies.as_ref();

        deps.and_then(find_in_deps)
            .or_else(|| deps_dev.and_then(find_in_deps))
    }
    pub fn get_path(&self, name: &str) -> syn::Path {
        self.maybe_get_path(name)
            .unwrap_or_else(|| Self::parse_str(name))
    }

    pub fn parse_str<T: syn::parse::Parse>(path: &str) -> T {
        syn::parse(path.parse::<TokenStream>().unwrap()).unwrap()
    }
}

/// Derive a label trait
///
/// # Args
///
/// - `input`: The [`syn::DeriveInput`] for struct that is deriving the label trait
/// - `trait_path`: The path [`syn::Path`] to the label trait
pub fn derive_label(
    input: syn::DeriveInput,
    trait_path: &syn::Path,
    boxed_type_path: &syn::Path,
) -> TokenStream {
    let ident = input.ident;

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let mut where_clause = where_clause.cloned().unwrap_or_else(|| syn::WhereClause {
        where_token: Default::default(),
        predicates: Default::default(),
    });
    where_clause.predicates.push(syn::parse2(quote! { Self: Eq + ::std::fmt::Debug + ::std::hash::Hash + Clone + Send + Sync + 'static }).unwrap());

    (quote! {
        impl #impl_generics #trait_path for #ident #ty_generics #where_clause {
            fn dyn_clone(&self) -> #boxed_type_path {
                #boxed_type_path::new(std::boxed::Box::new(std::clone::Clone::clone(self)))
            }
        }
    })
    .into()
}

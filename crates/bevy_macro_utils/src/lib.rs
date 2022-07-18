extern crate proc_macro;

mod attrs;
mod shape;
mod symbol;

pub use attrs::*;
pub use shape::*;
pub use symbol::*;

use proc_macro::TokenStream;
use quote::quote;
use std::{env, path::PathBuf};
use toml::{map::Map, Value};

pub struct BevyManifest {
    manifest: Map<String, Value>,
}

impl Default for BevyManifest {
    fn default() -> Self {
        Self {
            manifest: env::var_os("CARGO_MANIFEST_DIR")
                .map(PathBuf::from)
                .map(|mut path| {
                    path.push("Cargo.toml");
                    let manifest = std::fs::read_to_string(path).unwrap();
                    toml::from_str(&manifest).unwrap()
                })
                .unwrap(),
        }
    }
}

impl BevyManifest {
    pub fn maybe_get_path(&self, name: &str) -> Option<syn::Path> {
        const BEVY: &str = "bevy";
        const BEVY_INTERNAL: &str = "bevy_internal";

        fn dep_package(dep: &Value) -> Option<&str> {
            if dep.as_str().is_some() {
                None
            } else {
                dep.as_table()
                    .unwrap()
                    .get("package")
                    .map(|name| name.as_str().unwrap())
            }
        }

        let find_in_deps = |deps: &Map<String, Value>| -> Option<syn::Path> {
            let package = if let Some(dep) = deps.get(name) {
                return Some(Self::parse_str(dep_package(dep).unwrap_or(name)));
            } else if let Some(dep) = deps.get(BEVY) {
                dep_package(dep).unwrap_or(BEVY)
            } else if let Some(dep) = deps.get(BEVY_INTERNAL) {
                dep_package(dep).unwrap_or(BEVY_INTERNAL)
            } else {
                return None;
            };

            let mut path = Self::parse_str::<syn::Path>(package);
            if let Some(module) = name.strip_prefix("bevy_") {
                path.segments.push(Self::parse_str(module));
            }
            Some(path)
        };

        let deps = self
            .manifest
            .get("dependencies")
            .map(|deps| deps.as_table().unwrap());
        let deps_dev = self
            .manifest
            .get("dev-dependencies")
            .map(|deps| deps.as_table().unwrap());

        deps.and_then(find_in_deps)
            .or_else(|| deps_dev.and_then(find_in_deps))
    }

    /// Returns the path for the crate with the given name.
    ///
    /// This is a convenience method for constructing a [manifest] and
    /// calling the [`get_path`] method.
    ///
    /// This method should only be used where you just need the path and can't
    /// cache the [manifest]. If caching is possible, it's recommended to create
    /// the [manifest] yourself and use the [`get_path`] method.
    ///
    /// [`get_path`]: Self::get_path
    /// [manifest]: Self
    pub fn get_path_direct(name: &str) -> syn::Path {
        Self::default().get_path(name)
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
pub fn derive_label(input: syn::DeriveInput, trait_path: &syn::Path) -> TokenStream {
    // return true if the variant specified is an `ignore_fields` attribute
    fn is_ignore(attr: &syn::Attribute, attr_name: &str) -> bool {
        if attr.path.get_ident().as_ref().unwrap() != &attr_name {
            return false;
        }

        syn::custom_keyword!(ignore_fields);
        attr.parse_args_with(|input: syn::parse::ParseStream| {
            let ignore = input.parse::<Option<ignore_fields>>()?.is_some();
            Ok(ignore)
        })
        .unwrap()
    }

    let ident = input.ident;

    use convert_case::{Case, Casing};
    let trait_snake_case = trait_path
        .segments
        .last()
        .unwrap()
        .ident
        .to_string()
        .to_case(Case::Snake);

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let mut where_clause = where_clause.cloned().unwrap_or_else(|| syn::WhereClause {
        where_token: Default::default(),
        predicates: Default::default(),
    });
    where_clause
        .predicates
        .push(syn::parse2(quote! { Self: 'static }).unwrap());

    let as_str = match input.data {
        syn::Data::Struct(d) => {
            // Structs must either be fieldless, or explicitly ignore the fields.
            let ignore_fields = input.attrs.iter().any(|a| is_ignore(a, &trait_snake_case));
            if matches!(d.fields, syn::Fields::Unit) || ignore_fields {
                let lit = ident.to_string();
                quote! { #lit }
            } else {
                panic!("Labels cannot contain data, unless explicitly ignored with `#[{trait_snake_case}(ignore_fields)]`");
            }
        }
        syn::Data::Enum(d) => {
            let arms = d.variants.iter().map(|v| {
                // Variants must either be fieldless, or explicitly ignore the fields.
                let ignore_fields = v.attrs.iter().any(|a| is_ignore(a, &trait_snake_case));
                if matches!(v.fields, syn::Fields::Unit) | ignore_fields {
                    let mut path = syn::Path::from(ident.clone());
                    path.segments.push(v.ident.clone().into());
                    let lit = format!("{ident}::{}", v.ident.clone());
                    quote! { #path { .. } => #lit }
                } else {
                    panic!("Label variants cannot contain data, unless explicitly ignored with `#[{trait_snake_case}(ignore_fields)]`");
                }
            });
            quote! {
                match self {
                    #(#arms),*
                }
            }
        }
        syn::Data::Union(_) => panic!("Unions cannot be used as labels."),
    };

    (quote! {
        impl #impl_generics #trait_path for #ident #ty_generics #where_clause {
            fn as_str(&self) -> &'static str {
                #as_str
            }
        }
    })
    .into()
}

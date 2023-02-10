extern crate proc_macro;

mod attrs;
mod shape;
mod symbol;

pub use attrs::*;
pub use shape::*;
pub use symbol::*;

use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use std::{env, path::PathBuf};
use syn::spanned::Spanned;
use toml_edit::{Document, Item};

pub struct BevyManifest {
    manifest: Document,
}

impl Default for BevyManifest {
    fn default() -> Self {
        Self {
            manifest: env::var_os("CARGO_MANIFEST_DIR")
                .map(PathBuf::from)
                .map(|mut path| {
                    path.push("Cargo.toml");
                    let manifest = std::fs::read_to_string(path).unwrap();
                    manifest.parse::<Document>().unwrap()
                })
                .unwrap(),
        }
    }
}
const BEVY: &str = "bevy";
const BEVY_INTERNAL: &str = "bevy_internal";

impl BevyManifest {
    pub fn maybe_get_path(&self, name: &str) -> Option<syn::Path> {
        fn dep_package(dep: &Item) -> Option<&str> {
            if dep.as_str().is_some() {
                None
            } else {
                dep.get("package").map(|name| name.as_str().unwrap())
            }
        }

        let find_in_deps = |deps: &Item| -> Option<syn::Path> {
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

        let deps = self.manifest.get("dependencies");
        let deps_dev = self.manifest.get("dev-dependencies");

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

    pub fn get_subcrate(&self, subcrate: &str) -> Option<syn::Path> {
        self.maybe_get_path(BEVY)
            .map(|bevy_path| {
                let mut segments = bevy_path.segments;
                segments.push(BevyManifest::parse_str(subcrate));
                syn::Path {
                    leading_colon: None,
                    segments,
                }
            })
            .or_else(|| self.maybe_get_path(&format!("bevy_{subcrate}")))
    }
}

/// Derive a label trait
///
/// # Args
///
/// - `input`: The [`syn::DeriveInput`] for struct that is deriving the label trait
/// - `trait_path`: The path [`syn::Path`] to the label trait
pub fn derive_boxed_label(input: syn::DeriveInput, trait_path: &syn::Path) -> TokenStream {
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

    (quote! {
        impl #impl_generics #trait_path for #ident #ty_generics #where_clause {
            fn dyn_clone(&self) -> std::boxed::Box<dyn #trait_path> {
                std::boxed::Box::new(std::clone::Clone::clone(self))
            }
        }
    })
    .into()
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
    attr_name: &str,
) -> TokenStream {
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

    let ident = input.ident.clone();

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
            // see if the user tried to ignore fields incorrectly
            if let Some(attr) = d
                .fields
                .iter()
                .flat_map(|f| &f.attrs)
                .find(|a| is_ignore(a, attr_name))
            {
                let err_msg = format!("`#[{attr_name}(ignore_fields)]` cannot be applied to fields individually: add it to the struct declaration");
                return quote_spanned! {
                    attr.span() => compile_error!(#err_msg);
                }
                .into();
            }
            // Structs must either be fieldless, or explicitly ignore the fields.
            let ignore_fields = input.attrs.iter().any(|a| is_ignore(a, attr_name));
            if matches!(d.fields, syn::Fields::Unit) || ignore_fields {
                let lit = ident.to_string();
                quote! { #lit }
            } else {
                let err_msg = format!("Labels cannot contain data, unless explicitly ignored with `#[{attr_name}(ignore_fields)]`");
                return quote_spanned! {
                    d.fields.span() => compile_error!(#err_msg);
                }
                .into();
            }
        }
        syn::Data::Enum(d) => {
            // check if the user put #[label(ignore_fields)] in the wrong place
            if let Some(attr) = input.attrs.iter().find(|a| is_ignore(a, attr_name)) {
                let err_msg = format!("`#[{attr_name}(ignore_fields)]` can only be applied to enum variants or struct declarations");
                return quote_spanned! {
                    attr.span() => compile_error!(#err_msg);
                }
                .into();
            }
            let arms = d.variants.iter().map(|v| {
                // Variants must either be fieldless, or explicitly ignore the fields.
                let ignore_fields = v.attrs.iter().any(|a| is_ignore(a, attr_name));
                if matches!(v.fields, syn::Fields::Unit) | ignore_fields {
                    let mut path = syn::Path::from(ident.clone());
                    path.segments.push(v.ident.clone().into());
                    let lit = format!("{ident}::{}", v.ident.clone());
                    quote! { #path { .. } => #lit }
                } else {
                    let err_msg = format!("Label variants cannot contain data, unless explicitly ignored with `#[{attr_name}(ignore_fields)]`");
                    quote_spanned! {
                        v.fields.span() => _ => { compile_error!(#err_msg); }
                    }
                }
            });
            quote! {
                match self {
                    #(#arms),*
                }
            }
        }
        syn::Data::Union(_) => {
            return quote_spanned! {
                input.span() => compile_error!("Unions cannot be used as labels.");
            }
            .into();
        }
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

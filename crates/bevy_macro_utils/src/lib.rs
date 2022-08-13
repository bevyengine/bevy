extern crate proc_macro;

mod attrs;
mod shape;
mod symbol;

pub use attrs::*;
pub use shape::*;
pub use symbol::*;

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use std::{env, path::PathBuf};
use syn::spanned::Spanned;
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

/// A set of attributes defined on an item, variant, or field,
/// in the form e.g. `#[system_label(..)]`.
#[derive(Default)]
struct LabelAttrs {
    ignore_fields: Option<Span>,
}

impl LabelAttrs {
    /// Parses a list of attributes.
    ///
    /// Ignores any that aren't of the form `#[my_label(..)]`.
    /// Returns `Ok` if the iterator is empty.
    pub fn new<'a>(
        iter: impl IntoIterator<Item = &'a syn::Attribute>,
        attr_name: &str,
    ) -> syn::Result<Self> {
        let mut this = Self::default();
        for attr in iter {
            // If it's not of the form `#[my_label(..)]`, skip it.
            if attr.path.get_ident().as_ref().unwrap() != &attr_name {
                continue;
            }

            // Parse the argument/s to the attribute.
            attr.parse_args_with(|input: syn::parse::ParseStream| {
                loop {
                    syn::custom_keyword!(ignore_fields);

                    let next = input.lookahead1();
                    if next.peek(ignore_fields) {
                        let kw: ignore_fields = input.parse()?;
                        this.ignore_fields = Some(kw.span);
                    } else {
                        return Err(next.error());
                    }

                    if input.is_empty() {
                        break;
                    }
                    let _comma: syn::Token![,] = input.parse()?;
                }
                Ok(())
            })?;
        }

        Ok(this)
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
    attr_name: &str,
) -> TokenStream {
    let item_attrs = match LabelAttrs::new(&input.attrs, attr_name) {
        Ok(a) => a,
        Err(e) => return e.into_compile_error().into(),
    };

    derive_named_label(input, &item_attrs, trait_path, attr_name)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn with_static_bound(where_clause: Option<&syn::WhereClause>) -> syn::WhereClause {
    let mut where_clause = where_clause.cloned().unwrap_or_else(|| syn::WhereClause {
        where_token: Default::default(),
        predicates: Default::default(),
    });
    where_clause
        .predicates
        .push(syn::parse2(quote! { Self: 'static }).unwrap());
    where_clause
}

fn derive_named_label(
    input: syn::DeriveInput,
    item_attrs: &LabelAttrs,
    trait_path: &syn::Path,
    attr_name: &str,
) -> syn::Result<TokenStream2> {
    let ident = input.ident.clone();
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let where_clause = with_static_bound(where_clause);

    let (data, fmt) = match input.data {
        syn::Data::Struct(d) => {
            let all_field_attrs =
                LabelAttrs::new(d.fields.iter().flat_map(|f| &f.attrs), attr_name)?;
            // see if the user tried to ignore fields incorrectly
            if let Some(attr) = all_field_attrs.ignore_fields {
                let err_msg = format!(
                    r#"`#[{attr_name}(ignore_fields)]` cannot be applied to fields individually:
                    try adding it to the struct declaration"#
                );
                return Err(syn::Error::new(attr, err_msg));
            }
            // Structs must either be fieldless, or explicitly ignore the fields.
            let ignore_fields = item_attrs.ignore_fields.is_some();
            if d.fields.is_empty() || ignore_fields {
                let lit = ident.to_string();
                let data = quote! { 0 };
                let as_str = quote! { write!(f, #lit) };
                (data, as_str)
            } else {
                let err_msg = format!("Labels cannot contain data, unless explicitly ignored with `#[{attr_name}(ignore_fields)]`");
                return Err(syn::Error::new(d.fields.span(), err_msg));
            }
        }
        syn::Data::Enum(d) => {
            // check if the user put #[label(ignore_fields)] in the wrong place
            if let Some(attr) = item_attrs.ignore_fields {
                let err_msg = format!("`#[{attr_name}(ignore_fields)]` can only be applied to enum variants or struct declarations");
                return Err(syn::Error::new(attr, err_msg));
            }

            let mut data_arms = Vec::with_capacity(d.variants.len());
            let mut fmt_arms = Vec::with_capacity(d.variants.len());

            for (i, v) in d.variants.iter().enumerate() {
                let v_attrs = LabelAttrs::new(&v.attrs, attr_name)?;
                // Variants must either be fieldless, or explicitly ignore the fields.
                let ignore_fields = v_attrs.ignore_fields.is_some();
                if v.fields.is_empty() || ignore_fields {
                    let mut path = syn::Path::from(ident.clone());
                    path.segments.push(v.ident.clone().into());

                    let i = i as u64;
                    data_arms.push(quote! { #path { .. } => #i });

                    let lit = format!("{ident}::{}", v.ident.clone());
                    fmt_arms.push(quote! { #i => { write!(f, #lit) } });
                } else {
                    let err_msg = format!("Label variants cannot contain data, unless explicitly ignored with `#[{attr_name}(ignore_fields)]`");
                    return Err(syn::Error::new(v.fields.span(), err_msg));
                }
            }

            let data = quote! {
                match self {
                    #(#data_arms),*
                }
            };
            let fmt = quote! {
                match data {
                    #(#fmt_arms),*
                    _ => Err(::std::fmt::Error),
                }
            };
            (data, fmt)
        }
        syn::Data::Union(_) => {
            let err_msg = format!(
                "Unions cannot be used as labels, unless marked with `#[{attr_name}(intern)]`."
            );
            return Err(syn::Error::new(input.span(), err_msg));
        }
    };

    // Formatting for generics
    let generics = input.generics.params.iter();
    let fmt_generics = generics
        .filter_map(|p| match p {
            syn::GenericParam::Type(ty) => Some({
                let ident = &ty.ident;
                quote! { write!(f, "{}", ::std::any::type_name::<#ident>()) }
            }),
            syn::GenericParam::Const(c) => Some({
                let ident = &c.ident;
                quote! { write!(f, "{:?}", { #ident }) }
            }),
            _ => None,
        })
        .reduce(|a, b| quote! { #a?; write!(f, ", ")?; #b })
        .map(|x| quote! { write!(f, "::<")?; #x?; write!(f, ">")?; });

    Ok(quote! {
        impl #impl_generics #trait_path for #ident #ty_generics #where_clause {
            #[inline]
            fn data(&self) -> u64 {
                #data
            }
            fn fmt(data: u64, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                #fmt?;
                #fmt_generics
                Ok(())
            }
        }
    })
}

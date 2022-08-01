//! Contains code related to field attributes for reflected types.
//!
//! A field attribute is an attribute which applies to particular field or variant
//! as opposed to an entire struct or enum. An example of such an attribute is
//! the derive helper attribute for `Reflect`, which looks like: `#[reflect(ignore)]`.

use crate::REFLECT_ATTRIBUTE_NAME;
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Attribute, Lit, Meta, NestedMeta};

pub(crate) static IGNORE_ATTR: &str = "ignore";
pub(crate) static DEFAULT_ATTR: &str = "default";

/// A container for attributes defined on a reflected type's field.
#[derive(Default)]
pub(crate) struct ReflectFieldAttr {
    /// Determines if this field should be ignored.
    pub ignore: bool,
    /// Sets the default behavior of this field.
    pub default: DefaultBehavior,
}

/// Controls how the default value is determined for a field.
#[derive(Default)]
pub(crate) enum DefaultBehavior {
    /// Field is required.
    #[default]
    Required,
    /// Field can be defaulted using `Default::default()`.
    Default,
    /// Field can be created using the given function name.
    ///
    /// This assumes the function is in scope, is callable with zero arguments,
    /// and returns the expected type.
    Func(syn::ExprPath),
}

/// Parse all field attributes marked "reflect" (such as `#[reflect(ignore)]`).
pub(crate) fn parse_field_attrs(attrs: &[Attribute]) -> Result<ReflectFieldAttr, syn::Error> {
    let mut args = ReflectFieldAttr::default();
    let mut errors: Option<syn::Error> = None;

    let attrs = attrs
        .iter()
        .filter(|a| a.path.is_ident(REFLECT_ATTRIBUTE_NAME));
    for attr in attrs {
        let meta = attr.parse_meta()?;
        if let Err(err) = parse_meta(&mut args, &meta) {
            if let Some(ref mut error) = errors {
                error.combine(err);
            } else {
                errors = Some(err);
            }
        }
    }

    if let Some(error) = errors {
        Err(error)
    } else {
        Ok(args)
    }
}

/// Recursively parses attribute metadata for things like `#[reflect(ignore)]` and `#[reflect(default = "foo")]`
fn parse_meta(args: &mut ReflectFieldAttr, meta: &Meta) -> Result<(), syn::Error> {
    match meta {
        Meta::Path(path) if path.is_ident(IGNORE_ATTR) => {
            args.ignore = true;
            Ok(())
        }
        Meta::Path(path) if path.is_ident(DEFAULT_ATTR) => {
            args.default = DefaultBehavior::Default;
            Ok(())
        }
        Meta::Path(path) => Err(syn::Error::new(
            path.span(),
            format!("unknown attribute parameter: {}", path.to_token_stream()),
        )),
        Meta::NameValue(pair) if pair.path.is_ident(DEFAULT_ATTR) => {
            let lit = &pair.lit;
            match lit {
                Lit::Str(lit_str) => {
                    args.default = DefaultBehavior::Func(lit_str.parse()?);
                    Ok(())
                }
                err => {
                    Err(syn::Error::new(
                        err.span(),
                        format!("expected a string literal containing the name of a function, but found: {}", err.to_token_stream()),
                    ))
                }
            }
        }
        Meta::NameValue(pair) => {
            let path = &pair.path;
            Err(syn::Error::new(
                path.span(),
                format!("unknown attribute parameter: {}", path.to_token_stream()),
            ))
        }
        Meta::List(list) if !list.path.is_ident(REFLECT_ATTRIBUTE_NAME) => {
            Err(syn::Error::new(list.path.span(), "unexpected property"))
        }
        Meta::List(list) => {
            for nested in &list.nested {
                if let NestedMeta::Meta(meta) = nested {
                    parse_meta(args, meta)?;
                }
            }
            Ok(())
        }
    }
}

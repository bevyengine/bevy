//! Contains code related to field attributes for reflected types.
//!
//! A field attribute is an attribute which applies to particular field or variant
//! as opposed to an entire struct or enum. An example of such an attribute is
//! the derive helper attribute for `Reflect`, which looks like: `#[reflect(ignore)]`.

use crate::REFLECT_ATTRIBUTE_NAME;
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Attribute, Expr, ExprLit, Lit, Meta};

pub(crate) static IGNORE_SERIALIZATION_ATTR: &str = "skip_serializing";
pub(crate) static IGNORE_ALL_ATTR: &str = "ignore";

pub(crate) static DEFAULT_ATTR: &str = "default";

/// Stores data about if the field should be visible via the Reflect and serialization interfaces
///
/// Note the relationship between serialization and reflection is such that a member must be reflected in order to be serialized.
/// In boolean logic this is described as: `is_serialized -> is_reflected`, this means we can reflect something without serializing it but not the other way round.
/// The `is_reflected` predicate is provided as `self.is_active()`
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReflectIgnoreBehavior {
    /// Don't ignore, appear to all systems
    #[default]
    None,
    /// Ignore when serializing but not when reflecting
    IgnoreSerialization,
    /// Ignore both when serializing and reflecting
    IgnoreAlways,
}

impl ReflectIgnoreBehavior {
    /// Returns `true` if the ignoring behavior implies member is included in the reflection API, and false otherwise.
    pub fn is_active(self) -> bool {
        match self {
            ReflectIgnoreBehavior::None | ReflectIgnoreBehavior::IgnoreSerialization => true,
            ReflectIgnoreBehavior::IgnoreAlways => false,
        }
    }

    /// The exact logical opposite of `self.is_active()` returns true iff this member is not part of the reflection API whatsoever (neither serialized nor reflected)
    pub fn is_ignored(self) -> bool {
        !self.is_active()
    }
}

/// A container for attributes defined on a reflected type's field.
#[derive(Default, Clone)]
pub(crate) struct ReflectFieldAttr {
    /// Determines how this field should be ignored if at all.
    pub ignore: ReflectIgnoreBehavior,
    /// Sets the default behavior of this field.
    pub default: DefaultBehavior,
}

/// Controls how the default value is determined for a field.
#[derive(Default, Clone)]
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
        .filter(|a| a.path().is_ident(REFLECT_ATTRIBUTE_NAME));
    for attr in attrs {
        if let Err(err) = parse_meta(&mut args, &attr.meta) {
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
        Meta::Path(path) if path.is_ident(IGNORE_SERIALIZATION_ATTR) => {
            (args.ignore == ReflectIgnoreBehavior::None)
                .then(|| args.ignore = ReflectIgnoreBehavior::IgnoreSerialization)
                .ok_or_else(|| syn::Error::new_spanned(path, format!("Only one of ['{IGNORE_SERIALIZATION_ATTR}','{IGNORE_ALL_ATTR}'] is allowed")))
        }
        Meta::Path(path) if path.is_ident(IGNORE_ALL_ATTR) => {
            (args.ignore == ReflectIgnoreBehavior::None)
                .then(|| args.ignore = ReflectIgnoreBehavior::IgnoreAlways)
                .ok_or_else(|| syn::Error::new_spanned(path, format!("Only one of ['{IGNORE_SERIALIZATION_ATTR}','{IGNORE_ALL_ATTR}'] is allowed")))
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
            if let Expr::Lit(ExprLit {lit: Lit::Str(lit_str), ..}) = &pair.value {
                args.default = DefaultBehavior::Func(lit_str.parse()?);
                Ok(())
            }
            else {
                Err(syn::Error::new(
                    pair.span(),
                    format!("expected a string literal containing the name of a function, but found: {}", pair.to_token_stream()),
                ))?
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
            if let Ok(meta) = list.parse_args() {
                parse_meta(args, &meta)?;
            }
            Ok(())
        }
    }
}

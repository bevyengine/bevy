//! Contains code related to field attributes for reflected types.
//!
//! A field attribute is an attribute which applies to particular field or variant
//! as opposed to an entire struct or enum. An example of such an attribute is
//! the derive helper attribute for `Reflect`, which looks like: `#[reflect(ignore)]`.

use crate::{REFLECT_ATTRIBUTE_NAME, REFLECT_VALUE_ATTRIBUTE_NAME};
use proc_macro2::Span;
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Attribute, Lit, Meta, NestedMeta, Path};

pub(crate) const DEFAULT_ATTR: &str = "default";
pub(crate) const IGNORE_ALL_ATTR: &str = "ignore";
pub(crate) const IGNORE_SERIALIZATION_ATTR: &str = "skip_serializing";

// The attributes allowed on a field (in alphabetical order)
const ALLOWED_FIELD_ATTRS: &[&str] = &[DEFAULT_ATTR, IGNORE_ALL_ATTR, IGNORE_SERIALIZATION_ATTR];

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
    /// Returns `true` if the ignoring behaviour implies member is included in the reflection API, and false otherwise.
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
#[derive(Default)]
pub(crate) struct ReflectFieldAttr {
    /// Determines how this field should be ignored if at all.
    pub ignore: ReflectIgnoreBehavior,
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
pub(crate) fn parse_field_attrs(
    attrs: &[Attribute],
    is_variant: bool,
) -> Result<ReflectFieldAttr, syn::Error> {
    let mut args = ReflectFieldAttr::default();
    let mut errors: Option<syn::Error> = None;

    let mut combine_error = |err| {
        if let Some(ref mut error) = errors {
            error.combine(err);
        } else {
            errors = Some(err);
        }
    };

    for attr in attrs {
        let attr_ident = match attr.path.get_ident() {
            Some(ident) => ident.to_string(),
            None => continue,
        };

        match attr_ident.as_str() {
            REFLECT_ATTRIBUTE_NAME => {}
            REFLECT_VALUE_ATTRIBUTE_NAME => combine_error(syn::Error::new(
                attr.path.span(),
                format!(
                    "cannot use `{}` on a {}. Did you mean to use `{}`?",
                    REFLECT_VALUE_ATTRIBUTE_NAME,
                    if is_variant { "variant" } else { "field" },
                    REFLECT_ATTRIBUTE_NAME
                ),
            )),
            _ => continue,
        }

        let meta = attr.parse_meta()?;
        if let Err(err) = parse_meta(&mut args, &meta, is_variant) {
            combine_error(err);
        }
    }

    if let Some(error) = errors {
        Err(error)
    } else {
        Ok(args)
    }
}

/// Recursively parses attribute metadata for things like `#[reflect(ignore)]` and `#[reflect(default = "foo")]`
fn parse_meta(
    args: &mut ReflectFieldAttr,
    meta: &Meta,
    is_variant: bool,
) -> Result<(), syn::Error> {
    match meta {
        // Handles `#[reflect(skip_serializing)]`
        Meta::Path(path) if path.is_ident(IGNORE_SERIALIZATION_ATTR) => {
            deny_variant_attr(path.span(), IGNORE_SERIALIZATION_ATTR, is_variant)?;

            (args.ignore == ReflectIgnoreBehavior::None)
                .then(|| args.ignore = ReflectIgnoreBehavior::IgnoreSerialization)
                .ok_or_else(|| syn::Error::new_spanned(path, format!("only one of [\"{IGNORE_ALL_ATTR}\", \"{IGNORE_SERIALIZATION_ATTR}\"] is allowed")))
        }
        // Handles `#[reflect(ignore)]`
        Meta::Path(path) if path.is_ident(IGNORE_ALL_ATTR) => {
            deny_variant_attr(path.span(), IGNORE_ALL_ATTR, is_variant)?;

            (args.ignore == ReflectIgnoreBehavior::None)
                .then(|| args.ignore = ReflectIgnoreBehavior::IgnoreAlways)
                .ok_or_else(|| syn::Error::new_spanned(path, format!("only one of [\"{IGNORE_ALL_ATTR}\", \"{IGNORE_SERIALIZATION_ATTR}\"] is allowed")))
        }
        // Handles `#[reflect(default)]`
        Meta::Path(path) if path.is_ident(DEFAULT_ATTR) => {
            deny_variant_attr(path.span(), DEFAULT_ATTR, is_variant)?;

            args.default = DefaultBehavior::Default;
            Ok(())
        }
        // Handles `#[reflect(default = "foo")]`
        Meta::NameValue(pair) if pair.path.is_ident(DEFAULT_ATTR) => {
            deny_variant_attr(pair.path.span(), DEFAULT_ATTR, is_variant)?;

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
        // Handles `#[reflect(ignore, default = "foo", ...)]`
        Meta::List(list) => {
            for nested in &list.nested {
                if let NestedMeta::Meta(meta) = nested {
                    parse_meta(args, meta, is_variant)?;
                }
            }
            Ok(())
        }
        // === Invalid Attribute === //
        Meta::Path(path) => Err(unknown_attr(path, is_variant)),
        Meta::NameValue(pair) => {
            let path = &pair.path;
            Err(unknown_attr(path, is_variant))
        }
    }
}

/// Returns the generated error for an unknown attribute.
fn unknown_attr(path: &Path, is_variant: bool) -> syn::Error {
    if is_variant {
        syn::Error::new(
            path.span(),
            format!(
                "unknown variant attribute: \"{}\" (note: variants do not currently support any reflect attributes)",
                path.to_token_stream(),
            ),
        )
    } else {
        syn::Error::new(
            path.span(),
            format!(
                "unknown field attribute: \"{}\", expected one of {:?}",
                path.to_token_stream(),
                ALLOWED_FIELD_ATTRS
            ),
        )
    }
}

/// Returns an error for the given attribute if `is_variant` is true.
fn deny_variant_attr(span: Span, attr: &str, is_variant: bool) -> Result<(), syn::Error> {
    if is_variant {
        Err(syn::Error::new(
            span,
            format!("cannot use reflect attribute \"{}\" on enum variant", attr),
        ))
    } else {
        Ok(())
    }
}

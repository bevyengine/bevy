//! Contains code related to field attributes for reflected types.
//!
//! A field attribute is an attribute which applies to particular field or variant
//! as opposed to an entire struct or enum. An example of such an attribute is
//! the derive helper attribute for `Reflect`, which looks like: `#[reflect(ignore)]`.

use crate::REFLECT_ATTRIBUTE_NAME;
use proc_macro2::Span;
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Attribute, Lit, Meta, NestedMeta};

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
    /// Returns `true` if the ignoring behaviour implies member is included in the reflection API, and false otherwise.
    pub fn is_active(self) -> bool {
        match self {
            ReflectIgnoreBehavior::None | ReflectIgnoreBehavior::IgnoreSerialization => true,
            ReflectIgnoreBehavior::IgnoreAlways => false,
        }
    }

    /// The exact logical opposite of `self.is_active()` returns true iff this member is not part of the reflection API whatsover (neither serialized nor reflected)
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

/// Helper struct for parsing field attributes on structs, tuple structs, and enum variants.
pub(crate) struct ReflectFieldAttrParser {
    /// Indicates whether the fields being parsed are part of an enum variant.
    is_variant: bool,
    /// The [`Span`] for the last `#[reflect(ignore)]` attribute, if any.
    last_ignored: Option<Span>,
    /// The [`Span`] for the last `#[reflect(skip_serializing)]` attribute, if any.
    last_skipped: Option<Span>,
}

impl ReflectFieldAttrParser {
    /// Create a new parser for struct and tuple struct fields.
    pub fn new_struct() -> Self {
        Self {
            is_variant: false,
            last_ignored: None,
            last_skipped: None,
        }
    }

    /// Create a new parser for enum variant struct fields.
    pub fn new_enum_variant() -> Self {
        Self {
            is_variant: true,
            last_ignored: None,
            last_skipped: None,
        }
    }

    /// Parse all field attributes marked "reflect" (such as `#[reflect(ignore)]`).
    pub fn parse(&mut self, attrs: &[Attribute]) -> Result<ReflectFieldAttr, syn::Error> {
        let mut args = ReflectFieldAttr::default();
        let mut errors: Option<syn::Error> = None;

        let attrs = attrs
            .iter()
            .filter(|a| a.path.is_ident(REFLECT_ATTRIBUTE_NAME));
        for attr in attrs {
            let meta = attr.parse_meta()?;
            if let Err(err) = self.parse_meta(&mut args, &meta) {
                Self::combine_error(err, &mut errors);
            }
        }

        self.check_ignore_order(&args, &mut errors);
        self.check_skip_order(&args, &mut errors);

        if let Some(error) = errors {
            Err(error)
        } else {
            Ok(args)
        }
    }

    /// Recursively parses attribute metadata for things like `#[reflect(ignore)]` and `#[reflect(default = "foo")]`
    fn parse_meta(&mut self, args: &mut ReflectFieldAttr, meta: &Meta) -> Result<(), syn::Error> {
        match meta {
            Meta::Path(path) if path.is_ident(IGNORE_SERIALIZATION_ATTR) => {
                if args.ignore == ReflectIgnoreBehavior::None {
                    args.ignore = ReflectIgnoreBehavior::IgnoreSerialization;
                    self.last_skipped = Some(path.span());
                    Ok(())
                } else {
                    Err(syn::Error::new_spanned(path, format!("only one of ['{IGNORE_SERIALIZATION_ATTR}','{IGNORE_ALL_ATTR}'] is allowed")))
                }
            }
            Meta::Path(path) if path.is_ident(IGNORE_ALL_ATTR) => {
                if args.ignore == ReflectIgnoreBehavior::None {
                    args.ignore = ReflectIgnoreBehavior::IgnoreAlways;
                    self.last_ignored = Some(path.span());
                    Ok(())
                } else {
                    Err(syn::Error::new_spanned(path, format!("only one of ['{IGNORE_SERIALIZATION_ATTR}','{IGNORE_ALL_ATTR}'] is allowed")))
                }
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
                        self.parse_meta(args, meta)?;
                    }
                }
                Ok(())
            }
        }
    }

    /// Verifies `#[reflect(ignore)]` attributes are always last in the type definition.
    fn check_ignore_order(&self, args: &ReflectFieldAttr, errors: &mut Option<syn::Error>) {
        if args.ignore.is_active() {
            if let Some(span) = self.last_ignored {
                let message = if self.is_variant {
                    format!("fields marked with `#[reflect({IGNORE_ALL_ATTR})]` must come last in variant definition")
                } else {
                    format!("fields marked with `#[reflect({IGNORE_ALL_ATTR})]` must come last in type definition")
                };
                Self::combine_error(syn::Error::new(span, message), errors);
            }
        }
    }

    /// Verifies `#[reflect(skip_serializing)]` attributes are always last in the type definition,
    /// but before `#[reflect(ignore)]` attributes.
    fn check_skip_order(&self, args: &ReflectFieldAttr, errors: &mut Option<syn::Error>) {
        if args.ignore == ReflectIgnoreBehavior::None {
            if let Some(span) = self.last_skipped {
                let message = if self.is_variant {
                    format!("fields marked with `#[reflect({IGNORE_SERIALIZATION_ATTR})]` must come last in variant definition (but before any fields marked `#[reflect({IGNORE_ALL_ATTR})]`)")
                } else {
                    format!("fields marked with `#[reflect({IGNORE_SERIALIZATION_ATTR})]` must come last in type definition (but before any fields marked `#[reflect({IGNORE_ALL_ATTR})]`)")
                };
                Self::combine_error(syn::Error::new(span, message), errors);
            }
        }
    }

    /// Set or combine the given error into an optionally existing error.
    fn combine_error(err: syn::Error, errors: &mut Option<syn::Error>) {
        if let Some(error) = errors {
            error.combine(err);
        } else {
            *errors = Some(err);
        }
    }
}

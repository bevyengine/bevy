//! Contains code related to container attributes for reflected types.
//!
//! A container attribute is an attribute which applies to an entire struct or enum
//! as opposed to a particular field or variant. An example of such an attribute is
//! the derive helper attribute for `Reflect`, which looks like:
//! `#[reflect(partial_eq, Default, ...)]` and `#[reflect_value(PartialEq, Default, ...)]`.

use crate::fq_std::{FQAny, FQDefault, FQOption};
use crate::utility;
use proc_macro2::Span;
use quote::quote_spanned;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{parse_str, Lit, Meta, NestedMeta, Path};

// The "special" trait-like idents that are used internally for reflection.
// Received via attributes like `#[reflect(partial_eq, hash, ...)]`
const DEBUG_ATTR: &str = "debug";
const PARTIAL_EQ_ATTR: &str = "partial_eq";
const HASH_ATTR: &str = "hash";

/// A marker for special trait-like ident implementations registered via the `Reflect` derive macro.
#[derive(Clone, Default)]
pub(crate) enum TraitLikeImpl {
    /// The ident is not registered as implemented.
    #[default]
    NotImplemented,

    /// The ident is registered as implemented.
    Implemented(Span),

    /// The ident is registered with a custom function rather than the trait's implementation.
    Custom(Path, Span),
}

impl TraitLikeImpl {
    /// Merges this [`TraitLikeImpl`] with another.
    ///
    /// Returns whichever value is not [`TraitLikeImpl::NotImplemented`].
    /// If both values are [`TraitLikeImpl::NotImplemented`], then that is returned.
    /// Otherwise, an error is returned if neither value is [`TraitLikeImpl::NotImplemented`].
    pub fn merge(self, other: TraitLikeImpl) -> Result<TraitLikeImpl, syn::Error> {
        match (self, other) {
            (TraitLikeImpl::NotImplemented, value) | (value, TraitLikeImpl::NotImplemented) => {
                Ok(value)
            }
            (_, TraitLikeImpl::Implemented(span) | TraitLikeImpl::Custom(_, span)) => {
                Err(syn::Error::new(span, "conflicting type data registration"))
            }
        }
    }
}

/// A collection of traits that have been registered for a reflected type.
///
/// This keeps track of a few traits that are utilized internally for reflection
/// (we'll call these traits _special trait-like idents_ within this context), but it
/// will also keep track of all registered traits. Traits are registered as part of the
/// `Reflect` derive macro using the helper attribute: `#[reflect(...)]`.
///
/// The list of special trait-like idents are as follows:
/// * `debug`
/// * `hash`
/// * `partial_eq`
///
/// When registering a trait, there are a few things to keep in mind:
/// * Traits must have a valid `Reflect{}` struct in scope. For example, `Default`
///   needs `bevy_reflect::prelude::ReflectDefault` in scope.
/// * Traits must be single path identifiers. This means you _must_ use `Default`
///   instead of `std::default::Default` (otherwise it will try to register `Reflectstd`!)
/// * A custom function may be supplied in place of an actual implementation
///   for the special traits (but still follows the same single-path identifier
///   rules as normal).
///
/// # Example
///
/// Registering the `Default` implementation:
///
/// ```ignore
/// // Import ReflectDefault so it's accessible by the derive macro
/// use bevy_reflect::prelude::ReflectDefault.
///
/// #[derive(Reflect, Default)]
/// #[reflect(Default)]
/// struct Foo;
/// ```
///
/// Registering `hash` using the `Hash` trait:
///
/// ```ignore
/// // `hash` is "special" and does not need (nor have) a ReflectHash struct
///
/// #[derive(Reflect, Hash)]
/// #[reflect(hash)]
/// struct Foo;
/// ```
///
/// Registering `hash` using a custom function:
///
/// ```ignore
/// // This function acts is out hash function and
/// // corresponds to the `Reflect::reflect_hash` method.
/// fn get_hash(foo: &Foo) -> Option<u64> {
///   Some(123)
/// }
///
/// #[derive(Reflect)]
/// // Register the custom hash function
/// #[reflect(hash = "get_hash")]
/// struct Foo;
/// ```
///
/// > __Note:__ Registering a custom function only works for special traits.
///
#[derive(Default, Clone)]
pub(crate) struct ReflectTraits {
    debug: TraitLikeImpl,
    hash: TraitLikeImpl,
    partial_eq: TraitLikeImpl,
    idents: Vec<Path>,
}

impl ReflectTraits {
    /// Create a new [`ReflectTraits`] instance from a set of nested metas.
    pub fn from_nested_metas(
        nested_metas: &Punctuated<NestedMeta, Comma>,
    ) -> Result<Self, syn::Error> {
        let mut traits = ReflectTraits::default();
        for nested_meta in nested_metas.iter() {
            match nested_meta {
                // Handles `#[reflect(hash = "custom_hash_fn")]`
                NestedMeta::Meta(Meta::NameValue(name_value)) => {
                    if let Lit::Str(lit) = &name_value.lit {
                        // This should be the path of the custom function
                        let path: Path = parse_str(&lit.value())?;
                        // Track the span where the trait is implemented for future errors
                        let span = lit.span();
                        let trait_func_ident = TraitLikeImpl::Custom(path, span);
                        match name_value
                            .path
                            .get_ident()
                            .map(ToString::to_string)
                            .as_deref()
                        {
                            Some(DEBUG_ATTR) => {
                                traits.debug = traits.debug.merge(trait_func_ident)?;
                            }
                            Some(PARTIAL_EQ_ATTR) => {
                                traits.partial_eq = traits.partial_eq.merge(trait_func_ident)?;
                            }
                            Some(HASH_ATTR) => {
                                traits.hash = traits.hash.merge(trait_func_ident)?;
                            }
                            _ => {
                                return Err(syn::Error::new(
                                    span,
                                    "custom path literals can only be used for \"special\" idents.",
                                ))
                            }
                        }
                    }
                }
                // Handles `#[reflect( hash, Default, ... )]`
                NestedMeta::Meta(Meta::Path(path)) => {
                    // Track the span where the trait is implemented for future errors
                    let span = path.span();

                    match path.get_ident().map(ToString::to_string).as_deref() {
                        Some(DEBUG_ATTR) => {
                            traits.debug = traits.debug.merge(TraitLikeImpl::Implemented(span))?;
                        }
                        Some(PARTIAL_EQ_ATTR) => {
                            traits.partial_eq =
                                traits.partial_eq.merge(TraitLikeImpl::Implemented(span))?;
                        }
                        Some(HASH_ATTR) => {
                            traits.hash = traits.hash.merge(TraitLikeImpl::Implemented(span))?;
                        }
                        _ => {
                            // Create the reflect ident
                            // We set the span to the old ident so any compile errors point to that ident instead
                            let reflect_ident = utility::into_reflected_path(path.clone());
                            traits.idents.push(reflect_ident);
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(traits)
    }

    /// The list of reflected traits by their reflected ident (i.e. `ReflectDefault` for `Default`).
    pub fn paths(&self) -> &[Path] {
        &self.idents
    }

    /// Returns the implementation of `Reflect::reflect_hash` as a `TokenStream`.
    ///
    /// If `hash` was not registered, returns `None`.
    pub fn get_hash_impl(&self, bevy_reflect_path: &Path) -> Option<proc_macro2::TokenStream> {
        match &self.hash {
            &TraitLikeImpl::Implemented(span) => Some(quote_spanned! {span=>
                fn reflect_hash(&self) -> #FQOption<u64> {
                    use ::core::hash::{Hash, Hasher};
                    let mut hasher: #bevy_reflect_path::ReflectHasher = #FQDefault::default();
                    Hash::hash(&#FQAny::type_id(self), &mut hasher);
                    Hash::hash(self, &mut hasher);
                    #FQOption::Some(Hasher::finish(&hasher))
                }
            }),
            &TraitLikeImpl::Custom(ref impl_fn, span) => Some(quote_spanned! {span=>
                fn reflect_hash(&self) -> #FQOption<u64> {
                    #FQOption::Some(#impl_fn(self))
                }
            }),
            TraitLikeImpl::NotImplemented => None,
        }
    }

    /// Returns the implementation of `Reflect::reflect_partial_eq` as a `TokenStream`.
    ///
    /// If `partial_eq` was not registered, returns `None`.
    pub fn get_partial_eq_impl(
        &self,
        bevy_reflect_path: &Path,
    ) -> Option<proc_macro2::TokenStream> {
        match &self.partial_eq {
            &TraitLikeImpl::Implemented(span) => Some(quote_spanned! {span=>
                fn reflect_partial_eq(&self, value: &dyn #bevy_reflect_path::Reflect) -> #FQOption<bool> {
                    let value = <dyn #bevy_reflect_path::Reflect>::as_any(value);
                    if let #FQOption::Some(value) = <dyn #FQAny>::downcast_ref::<Self>(value) {
                        #FQOption::Some(::core::cmp::PartialEq::eq(self, value))
                    } else {
                        #FQOption::Some(false)
                    }
                }
            }),
            &TraitLikeImpl::Custom(ref impl_fn, span) => Some(quote_spanned! {span=>
                fn reflect_partial_eq(&self, value: &dyn #bevy_reflect_path::Reflect) -> #FQOption<bool> {
                    #FQOption::Some(#impl_fn(self, value))
                }
            }),
            TraitLikeImpl::NotImplemented => None,
        }
    }

    /// Returns the implementation of `Reflect::debug` as a `TokenStream`.
    ///
    /// If `debug` was not registered, returns `None`.
    pub fn get_debug_impl(&self) -> Option<proc_macro2::TokenStream> {
        match &self.debug {
            &TraitLikeImpl::Implemented(span) => Some(quote_spanned! {span=>
                fn debug(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    ::core::fmt::Debug::fmt(self, f)
                }
            }),
            &TraitLikeImpl::Custom(ref impl_fn, span) => Some(quote_spanned! {span=>
                fn debug(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    #impl_fn(self, f)
                }
            }),
            TraitLikeImpl::NotImplemented => None,
        }
    }

    /// Merges the trait implementations of this [`ReflectTraits`] with another one.
    ///
    /// An error is returned if the two [`ReflectTraits`] have conflicting implementations.
    pub fn merge(self, other: ReflectTraits) -> Result<Self, syn::Error> {
        Ok(ReflectTraits {
            debug: self.debug.merge(other.debug)?,
            hash: self.hash.merge(other.hash)?,
            partial_eq: self.partial_eq.merge(other.partial_eq)?,
            idents: {
                let mut idents = self.idents;
                idents.extend(other.idents);
                idents
            },
        })
    }
}

impl Parse for ReflectTraits {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let result = Punctuated::<NestedMeta, Comma>::parse_terminated(input)?;
        ReflectTraits::from_nested_metas(&result)
    }
}

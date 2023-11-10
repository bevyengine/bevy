//! Contains code related to container attributes for reflected types.
//!
//! A container attribute is an attribute which applies to an entire struct or enum
//! as opposed to a particular field or variant. An example of such an attribute is
//! the derive helper attribute for `Reflect`, which looks like:
//! `#[reflect(PartialEq, Default, ...)]` and `#[reflect_value(PartialEq, Default, ...)]`.

use crate::utility;
use bevy_macro_utils::fq_std::{FQAny, FQOption};
use proc_macro2::{Ident, Span};
use quote::quote_spanned;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{Expr, LitBool, Meta, Path};

// The "special" trait idents that are used internally for reflection.
// Received via attributes like `#[reflect(PartialEq, Hash, ...)]`
const DEBUG_ATTR: &str = "Debug";
const PARTIAL_EQ_ATTR: &str = "PartialEq";
const HASH_ATTR: &str = "Hash";

// The traits listed below are not considered "special" (i.e. they use the `ReflectMyTrait` syntax)
// but useful to know exist nonetheless
pub(crate) const REFLECT_DEFAULT: &str = "ReflectDefault";

// Attributes for `FromReflect` implementation
const FROM_REFLECT_ATTR: &str = "from_reflect";

// Attributes for `TypePath` implementation
const TYPE_PATH_ATTR: &str = "type_path";

// The error message to show when a trait/type is specified multiple times
const CONFLICTING_TYPE_DATA_MESSAGE: &str = "conflicting type data registration";

/// A marker for trait implementations registered via the `Reflect` derive macro.
#[derive(Clone, Default)]
pub(crate) enum TraitImpl {
    /// The trait is not registered as implemented.
    #[default]
    NotImplemented,

    /// The trait is registered as implemented.
    Implemented(Span),

    /// The trait is registered with a custom function rather than an actual implementation.
    Custom(Path, Span),
}

impl TraitImpl {
    /// Merges this [`TraitImpl`] with another.
    ///
    /// Update `self` with whichever value is not [`TraitImpl::NotImplemented`].
    /// If `other` is [`TraitImpl::NotImplemented`], then `self` is not modified.
    /// An error is returned if neither value is [`TraitImpl::NotImplemented`].
    pub fn merge(&mut self, other: TraitImpl) -> Result<(), syn::Error> {
        match (&self, other) {
            (TraitImpl::NotImplemented, value) => {
                *self = value;
                Ok(())
            }
            (_, TraitImpl::NotImplemented) => Ok(()),
            (_, TraitImpl::Implemented(span) | TraitImpl::Custom(_, span)) => {
                Err(syn::Error::new(span, CONFLICTING_TYPE_DATA_MESSAGE))
            }
        }
    }
}

/// A collection of attributes used for deriving `FromReflect`.
#[derive(Clone, Default)]
pub(crate) struct FromReflectAttrs {
    auto_derive: Option<LitBool>,
}

impl FromReflectAttrs {
    /// Returns true if `FromReflect` should be automatically derived as part of the `Reflect` derive.
    pub fn should_auto_derive(&self) -> bool {
        self.auto_derive
            .as_ref()
            .map(|lit| lit.value())
            .unwrap_or(true)
    }

    /// Merges this [`FromReflectAttrs`] with another.
    pub fn merge(&mut self, other: FromReflectAttrs) -> Result<(), syn::Error> {
        if let Some(new) = other.auto_derive {
            if let Some(existing) = &self.auto_derive {
                if existing.value() != new.value() {
                    return Err(syn::Error::new(
                        new.span(),
                        format!("`{FROM_REFLECT_ATTR}` already set to {}", existing.value()),
                    ));
                }
            } else {
                self.auto_derive = Some(new);
            }
        }

        Ok(())
    }
}

/// A collection of attributes used for deriving `TypePath` via the `Reflect` derive.
///
/// Note that this differs from the attributes used by the `TypePath` derive itself,
/// which look like `[type_path = "my_crate::foo"]`.
/// The attributes used by reflection take the form `#[reflect(type_path = false)]`.
///
/// These attributes should only be used for `TypePath` configuration specific to
/// deriving `Reflect`.
#[derive(Clone, Default)]
pub(crate) struct TypePathAttrs {
    auto_derive: Option<LitBool>,
}

impl TypePathAttrs {
    /// Returns true if `TypePath` should be automatically derived as part of the `Reflect` derive.
    pub fn should_auto_derive(&self) -> bool {
        self.auto_derive
            .as_ref()
            .map(|lit| lit.value())
            .unwrap_or(true)
    }

    /// Merges this [`TypePathAttrs`] with another.
    pub fn merge(&mut self, other: TypePathAttrs) -> Result<(), syn::Error> {
        if let Some(new) = other.auto_derive {
            if let Some(existing) = &self.auto_derive {
                if existing.value() != new.value() {
                    return Err(syn::Error::new(
                        new.span(),
                        format!("`{TYPE_PATH_ATTR}` already set to {}", existing.value()),
                    ));
                }
            } else {
                self.auto_derive = Some(new);
            }
        }

        Ok(())
    }
}

/// A collection of traits that have been registered for a reflected type.
///
/// This keeps track of a few traits that are utilized internally for reflection
/// (we'll call these traits _special traits_ within this context), but it
/// will also keep track of all registered traits. Traits are registered as part of the
/// `Reflect` derive macro using the helper attribute: `#[reflect(...)]`.
///
/// The list of special traits are as follows:
/// * `Debug`
/// * `Hash`
/// * `PartialEq`
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
/// Registering the `Hash` implementation:
///
/// ```ignore
/// // `Hash` is a "special trait" and does not need (nor have) a ReflectHash struct
///
/// #[derive(Reflect, Hash)]
/// #[reflect(Hash)]
/// struct Foo;
/// ```
///
/// Registering the `Hash` implementation using a custom function:
///
/// ```ignore
/// // This function acts as our `Hash` implementation and
/// // corresponds to the `Reflect::reflect_hash` method.
/// fn get_hash(foo: &Foo) -> Option<u64> {
///   Some(123)
/// }
///
/// #[derive(Reflect)]
/// // Register the custom `Hash` function
/// #[reflect(Hash(get_hash))]
/// struct Foo;
/// ```
///
/// > __Note:__ Registering a custom function only works for special traits.
///
#[derive(Default, Clone)]
pub(crate) struct ReflectTraits {
    debug: TraitImpl,
    hash: TraitImpl,
    partial_eq: TraitImpl,
    from_reflect_attrs: FromReflectAttrs,
    type_path_attrs: TypePathAttrs,
    idents: Vec<Ident>,
}

impl ReflectTraits {
    pub fn from_metas(
        metas: Punctuated<Meta, Comma>,
        is_from_reflect_derive: bool,
    ) -> Result<Self, syn::Error> {
        let mut traits = ReflectTraits::default();
        for meta in &metas {
            match meta {
                // Handles `#[reflect( Hash, Default, ... )]`
                Meta::Path(path) => {
                    // Get the first ident in the path (hopefully the path only contains one and not `std::hash::Hash`)
                    let Some(segment) = path.segments.iter().next() else {
                        continue;
                    };
                    let ident = &segment.ident;
                    let ident_name = ident.to_string();

                    // Track the span where the trait is implemented for future errors
                    let span = ident.span();

                    match ident_name.as_str() {
                        DEBUG_ATTR => {
                            traits.debug.merge(TraitImpl::Implemented(span))?;
                        }
                        PARTIAL_EQ_ATTR => {
                            traits.partial_eq.merge(TraitImpl::Implemented(span))?;
                        }
                        HASH_ATTR => {
                            traits.hash.merge(TraitImpl::Implemented(span))?;
                        }
                        // We only track reflected idents for traits not considered special
                        _ => {
                            // Create the reflect ident
                            // We set the span to the old ident so any compile errors point to that ident instead
                            let mut reflect_ident = utility::get_reflect_ident(&ident_name);
                            reflect_ident.set_span(span);

                            add_unique_ident(&mut traits.idents, reflect_ident)?;
                        }
                    }
                }
                // Handles `#[reflect( Hash(custom_hash_fn) )]`
                Meta::List(list) => {
                    // Get the first ident in the path (hopefully the path only contains one and not `std::hash::Hash`)
                    let Some(segment) = list.path.segments.iter().next() else {
                        continue;
                    };

                    let ident = segment.ident.to_string();

                    // Track the span where the trait is implemented for future errors
                    let span = ident.span();

                    list.parse_nested_meta(|meta| {
                        // This should be the path of the custom function
                        let trait_func_ident = TraitImpl::Custom(meta.path, span);
                        match ident.as_str() {
                            DEBUG_ATTR => {
                                traits.debug.merge(trait_func_ident)?;
                            }
                            PARTIAL_EQ_ATTR => {
                                traits.partial_eq.merge(trait_func_ident)?;
                            }
                            HASH_ATTR => {
                                traits.hash.merge(trait_func_ident)?;
                            }
                            _ => {
                                return Err(syn::Error::new(span, "Can only use custom functions for special traits (i.e. `Hash`, `PartialEq`, `Debug`)"));
                            }
                        }
                        Ok(())
                    })?;
                }
                Meta::NameValue(pair) => {
                    if pair.path.is_ident(FROM_REFLECT_ATTR) {
                        traits.from_reflect_attrs.auto_derive =
                            Some(extract_bool(&pair.value, |lit| {
                                // Override `lit` if this is a `FromReflect` derive.
                                // This typically means a user is opting out of the default implementation
                                // from the `Reflect` derive and using the `FromReflect` derive directly instead.
                                is_from_reflect_derive
                                    .then(|| LitBool::new(true, Span::call_site()))
                                    .unwrap_or_else(|| lit.clone())
                            })?);
                    } else if pair.path.is_ident(TYPE_PATH_ATTR) {
                        traits.type_path_attrs.auto_derive =
                            Some(extract_bool(&pair.value, Clone::clone)?);
                    } else {
                        return Err(syn::Error::new(pair.path.span(), "Unknown attribute"));
                    }
                }
            }
        }

        Ok(traits)
    }

    /// Returns true if the given reflected trait name (i.e. `ReflectDefault` for `Default`)
    /// is registered for this type.
    pub fn contains(&self, name: &str) -> bool {
        self.idents.iter().any(|ident| ident == name)
    }

    /// The list of reflected traits by their reflected ident (i.e. `ReflectDefault` for `Default`).
    pub fn idents(&self) -> &[Ident] {
        &self.idents
    }

    /// The `FromReflect` configuration found within `#[reflect(...)]` attributes on this type.
    #[allow(clippy::wrong_self_convention)]
    pub fn from_reflect_attrs(&self) -> &FromReflectAttrs {
        &self.from_reflect_attrs
    }

    /// The `TypePath` configuration found within `#[reflect(...)]` attributes on this type.
    pub fn type_path_attrs(&self) -> &TypePathAttrs {
        &self.type_path_attrs
    }

    /// Returns the implementation of `Reflect::reflect_hash` as a `TokenStream`.
    ///
    /// If `Hash` was not registered, returns `None`.
    pub fn get_hash_impl(&self, bevy_reflect_path: &Path) -> Option<proc_macro2::TokenStream> {
        match &self.hash {
            &TraitImpl::Implemented(span) => Some(quote_spanned! {span=>
                fn reflect_hash(&self) -> #FQOption<u64> {
                    use ::core::hash::{Hash, Hasher};
                    let mut hasher = #bevy_reflect_path::utility::reflect_hasher();
                    Hash::hash(&#FQAny::type_id(self), &mut hasher);
                    Hash::hash(self, &mut hasher);
                    #FQOption::Some(Hasher::finish(&hasher))
                }
            }),
            &TraitImpl::Custom(ref impl_fn, span) => Some(quote_spanned! {span=>
                fn reflect_hash(&self) -> #FQOption<u64> {
                    #FQOption::Some(#impl_fn(self))
                }
            }),
            TraitImpl::NotImplemented => None,
        }
    }

    /// Returns the implementation of `Reflect::reflect_partial_eq` as a `TokenStream`.
    ///
    /// If `PartialEq` was not registered, returns `None`.
    pub fn get_partial_eq_impl(
        &self,
        bevy_reflect_path: &Path,
    ) -> Option<proc_macro2::TokenStream> {
        match &self.partial_eq {
            &TraitImpl::Implemented(span) => Some(quote_spanned! {span=>
                fn reflect_partial_eq(&self, value: &dyn #bevy_reflect_path::Reflect) -> #FQOption<bool> {
                    let value = <dyn #bevy_reflect_path::Reflect>::as_any(value);
                    if let #FQOption::Some(value) = <dyn #FQAny>::downcast_ref::<Self>(value) {
                        #FQOption::Some(::core::cmp::PartialEq::eq(self, value))
                    } else {
                        #FQOption::Some(false)
                    }
                }
            }),
            &TraitImpl::Custom(ref impl_fn, span) => Some(quote_spanned! {span=>
                fn reflect_partial_eq(&self, value: &dyn #bevy_reflect_path::Reflect) -> #FQOption<bool> {
                    #FQOption::Some(#impl_fn(self, value))
                }
            }),
            TraitImpl::NotImplemented => None,
        }
    }

    /// Returns the implementation of `Reflect::debug` as a `TokenStream`.
    ///
    /// If `Debug` was not registered, returns `None`.
    pub fn get_debug_impl(&self) -> Option<proc_macro2::TokenStream> {
        match &self.debug {
            &TraitImpl::Implemented(span) => Some(quote_spanned! {span=>
                fn debug(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    ::core::fmt::Debug::fmt(self, f)
                }
            }),
            &TraitImpl::Custom(ref impl_fn, span) => Some(quote_spanned! {span=>
                fn debug(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    #impl_fn(self, f)
                }
            }),
            TraitImpl::NotImplemented => None,
        }
    }

    /// Merges the trait implementations of this [`ReflectTraits`] with another one.
    ///
    /// An error is returned if the two [`ReflectTraits`] have conflicting implementations.
    pub fn merge(&mut self, other: ReflectTraits) -> Result<(), syn::Error> {
        self.debug.merge(other.debug)?;
        self.hash.merge(other.hash)?;
        self.partial_eq.merge(other.partial_eq)?;
        self.from_reflect_attrs.merge(other.from_reflect_attrs)?;
        self.type_path_attrs.merge(other.type_path_attrs)?;
        for ident in other.idents {
            add_unique_ident(&mut self.idents, ident)?;
        }
        Ok(())
    }
}

impl Parse for ReflectTraits {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        ReflectTraits::from_metas(Punctuated::<Meta, Comma>::parse_terminated(input)?, false)
    }
}

/// Adds an identifier to a vector of identifiers if it is not already present.
///
/// Returns an error if the identifier already exists in the list.
fn add_unique_ident(idents: &mut Vec<Ident>, ident: Ident) -> Result<(), syn::Error> {
    let ident_name = ident.to_string();
    if idents.iter().any(|i| i == ident_name.as_str()) {
        return Err(syn::Error::new(ident.span(), CONFLICTING_TYPE_DATA_MESSAGE));
    }

    idents.push(ident);
    Ok(())
}

/// Extract a boolean value from an expression.
///
/// The mapper exists so that the caller can conditionally choose to use the given
/// value or supply their own.
fn extract_bool(
    value: &Expr,
    mut mapper: impl FnMut(&LitBool) -> LitBool,
) -> Result<LitBool, syn::Error> {
    match value {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Bool(lit),
            ..
        }) => Ok(mapper(lit)),
        _ => Err(syn::Error::new(value.span(), "Expected a boolean value")),
    }
}

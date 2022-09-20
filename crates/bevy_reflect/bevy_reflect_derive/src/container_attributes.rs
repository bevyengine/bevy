//! Contains code related to container attributes for reflected types.
//!
//! A container attribute is an attribute which applies to an entire struct or enum
//! as opposed to a particular field or variant. An example of such an attribute is
//! the derive helper attribute for `Reflect`, which looks like:
//! `#[reflect(PartialEq, Default, ...)]` and `#[reflect_value(PartialEq, Default, ...)]`.

use crate::utility;
use proc_macro2::Ident;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{Meta, NestedMeta, Path};

// The "special" trait idents that are used internally for reflection.
// Received via attributes like `#[reflect(PartialEq, Hash, ...)]`
const DEBUG_ATTR: &str = "Debug";
const PARTIAL_EQ_ATTR: &str = "PartialEq";
const HASH_ATTR: &str = "Hash";

// The traits listed below are not considered "special" (i.e. they use the `ReflectMyTrait` syntax)
// but useful to know exist nonetheless
pub(crate) const REFLECT_DEFAULT: &str = "ReflectDefault";

/// A marker for trait implementations registered via the `Reflect` derive macro.
#[derive(Clone, Default)]
pub(crate) enum TraitImpl {
    /// The trait is not registered as implemented.
    #[default]
    NotImplemented,
    /// The trait is registered as implemented.
    Implemented,

    // TODO: This can be made to use `ExprPath` instead of `Ident`, allowing for fully qualified paths to be used
    /// The trait is registered with a custom function rather than an actual implementation.
    Custom(Ident),
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
    idents: Vec<Ident>,
}

impl ReflectTraits {
    /// Create a new [`ReflectTraits`] instance from a set of nested metas.
    pub fn from_nested_metas(nested_metas: &Punctuated<NestedMeta, Comma>) -> Self {
        let mut traits = ReflectTraits::default();
        for nested_meta in nested_metas.iter() {
            match nested_meta {
                // Handles `#[reflect( Hash, Default, ... )]`
                NestedMeta::Meta(Meta::Path(path)) => {
                    // Get the first ident in the path (hopefully the path only contains one and not `std::hash::Hash`)
                    let ident = if let Some(segment) = path.segments.iter().next() {
                        segment.ident.to_string()
                    } else {
                        continue;
                    };

                    match ident.as_str() {
                        DEBUG_ATTR => traits.debug = TraitImpl::Implemented,
                        PARTIAL_EQ_ATTR => traits.partial_eq = TraitImpl::Implemented,
                        HASH_ATTR => traits.hash = TraitImpl::Implemented,
                        // We only track reflected idents for traits not considered special
                        _ => traits.idents.push(utility::get_reflect_ident(&ident)),
                    }
                }
                // Handles `#[reflect( Hash(custom_hash_fn) )]`
                NestedMeta::Meta(Meta::List(list)) => {
                    // Get the first ident in the path (hopefully the path only contains one and not `std::hash::Hash`)
                    let ident = if let Some(segment) = list.path.segments.iter().next() {
                        segment.ident.to_string()
                    } else {
                        continue;
                    };

                    let list_meta = list.nested.iter().next();
                    if let Some(NestedMeta::Meta(Meta::Path(path))) = list_meta {
                        if let Some(segment) = path.segments.iter().next() {
                            // This should be the ident of the custom function
                            let trait_func_ident = TraitImpl::Custom(segment.ident.clone());
                            match ident.as_str() {
                                DEBUG_ATTR => traits.debug = trait_func_ident,
                                PARTIAL_EQ_ATTR => traits.partial_eq = trait_func_ident,
                                HASH_ATTR => traits.hash = trait_func_ident,
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        traits
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

    /// Returns the implementation of `Reflect::reflect_hash` as a `TokenStream`.
    ///
    /// If `Hash` was not registered, returns `None`.
    pub fn get_hash_impl(&self, bevy_reflect_path: &Path) -> Option<proc_macro2::TokenStream> {
        match &self.hash {
            TraitImpl::Implemented => Some(quote! {
                fn reflect_hash(&self) -> Option<u64> {
                    use std::hash::{Hash, Hasher};
                    let mut hasher = #bevy_reflect_path::ReflectHasher::default();
                    Hash::hash(&std::any::Any::type_id(self), &mut hasher);
                    Hash::hash(self, &mut hasher);
                    Some(hasher.finish())
                }
            }),
            TraitImpl::Custom(impl_fn) => Some(quote! {
                fn reflect_hash(&self) -> Option<u64> {
                    Some(#impl_fn(self))
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
            TraitImpl::Implemented => Some(quote! {
                fn reflect_partial_eq(&self, value: &dyn #bevy_reflect_path::Reflect) -> Option<bool> {
                    let value = value.as_any();
                    if let Some(value) = value.downcast_ref::<Self>() {
                        Some(std::cmp::PartialEq::eq(self, value))
                    } else {
                        Some(false)
                    }
                }
            }),
            TraitImpl::Custom(impl_fn) => Some(quote! {
                fn reflect_partial_eq(&self, value: &dyn #bevy_reflect_path::Reflect) -> Option<bool> {
                    Some(#impl_fn(self, value))
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
            TraitImpl::Implemented => Some(quote! {
                fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    std::fmt::Debug::fmt(self, f)
                }
            }),
            TraitImpl::Custom(impl_fn) => Some(quote! {
                fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    #impl_fn(self, f)
                }
            }),
            TraitImpl::NotImplemented => None,
        }
    }
}

impl Parse for ReflectTraits {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let result = Punctuated::<NestedMeta, Comma>::parse_terminated(input)?;
        Ok(ReflectTraits::from_nested_metas(&result))
    }
}

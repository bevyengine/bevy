//! Traits used by label implementations

use std::{
    any::Any,
    hash::{Hash, Hasher},
};

/// An object safe version of [`Eq`]. This trait is automatically implemented
/// for any `'static` type that implements `Eq`.
pub trait DynEq: Any {
    /// Casts the type to `dyn Any`.
    fn as_any(&self) -> &dyn Any;

    /// This method tests for `self` and `other` values to be equal.
    ///
    /// Implementers should avoid returning `true` when the underlying types are
    /// not the same.
    fn dyn_eq(&self, other: &dyn DynEq) -> bool;
}

impl<T> DynEq for T
where
    T: Any + Eq,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn dyn_eq(&self, other: &dyn DynEq) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<T>() {
            return self == other;
        }
        false
    }
}

/// An object safe version of [`Hash`]. This trait is automatically implemented
/// for any `'static` type that implements `Hash`.
pub trait DynHash: DynEq {
    /// Casts the type to `dyn Any`.
    fn as_dyn_eq(&self) -> &dyn DynEq;

    /// Feeds this value into the given [`Hasher`].
    ///
    /// [`Hasher`]: std::hash::Hasher
    fn dyn_hash(&self, state: &mut dyn Hasher);
}

impl<T> DynHash for T
where
    T: DynEq + Hash,
{
    fn as_dyn_eq(&self) -> &dyn DynEq {
        self
    }

    fn dyn_hash(&self, mut state: &mut dyn Hasher) {
        T::hash(self, &mut state);
        self.type_id().hash(&mut state);
    }
}

/// Macro to define a new label trait
///
/// # Example
///
/// ```
/// # use bevy_utils::define_boxed_label;
/// define_boxed_label!(MyNewLabelTrait);
/// ```
#[macro_export]
macro_rules! define_boxed_label {
    ($label_trait_name:ident) => {
        /// A strongly-typed label.
        pub trait $label_trait_name: 'static + Send + Sync + ::std::fmt::Debug {
            /// Return's the [`TypeId`] of this label, or the ID of the
            /// wrapped label type for `Box<dyn
            #[doc = stringify!($label_trait_name)]
            /// >`
            ///
            /// [TypeId]: std::any::TypeId
            fn inner_type_id(&self) -> ::std::any::TypeId {
                std::any::TypeId::of::<Self>()
            }

            /// Clones this `
            #[doc = stringify!($label_trait_name)]
            /// `
            fn dyn_clone(&self) -> Box<dyn $label_trait_name>;

            /// Casts this value to a form where it can be compared with other type-erased values.
            fn as_dyn_eq(&self) -> &dyn ::bevy_utils::label::DynEq;

            /// Feeds this value into the given [`Hasher`].
            ///
            /// [`Hasher`]: std::hash::Hasher
            fn dyn_hash(&self, state: &mut dyn ::std::hash::Hasher);
        }

        impl PartialEq for dyn $label_trait_name {
            fn eq(&self, other: &Self) -> bool {
                self.as_dyn_eq().dyn_eq(other.as_dyn_eq())
            }
        }

        impl Eq for dyn $label_trait_name {}

        impl ::std::hash::Hash for dyn $label_trait_name {
            fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
                self.dyn_hash(state);
            }
        }

        impl ::std::convert::AsRef<dyn $label_trait_name> for dyn $label_trait_name {
            #[inline]
            fn as_ref(&self) -> &Self {
                self
            }
        }

        impl Clone for Box<dyn $label_trait_name> {
            fn clone(&self) -> Self {
                self.dyn_clone()
            }
        }

        impl $label_trait_name for Box<dyn $label_trait_name> {
            fn inner_type_id(&self) -> ::std::any::TypeId {
                (**self).inner_type_id()
            }

            fn dyn_clone(&self) -> Box<dyn $label_trait_name> {
                // Be explicit that we want to use the inner value
                // to avoid infinite recursion.
                (**self).dyn_clone()
            }

            fn as_dyn_eq(&self) -> &dyn ::bevy_utils::label::DynEq {
                (**self).as_dyn_eq()
            }

            fn dyn_hash(&self, state: &mut dyn ::std::hash::Hasher) {
                (**self).dyn_hash(state);
            }
        }
    };
}

/// Macro to define a new label trait
///
/// # Example
///
/// ```
/// # use bevy_utils::define_label;
/// define_label!(
///     /// A class of labels.
///     MyNewLabelTrait,
///     /// Identifies a value that implements `MyNewLabelTrait`.
///     MyNewLabelId,
/// );
/// ```
#[macro_export]
macro_rules! define_label {
    (
        $(#[$label_attr:meta])*
        $label_name:ident,

        $(#[$id_attr:meta])*
        $id_name:ident $(,)?
    ) => {
        $(#[$id_attr])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $id_name(::core::any::TypeId, &'static str);

        impl ::core::fmt::Debug for $id_name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                write!(f, "{}", self.1)
            }
        }

        $(#[$label_attr])*
        pub trait $label_name: Send + Sync + 'static {
            /// Converts this type into an opaque, strongly-typed label.
            fn as_label(&self) -> $id_name {
                let id = self.type_id();
                let label = self.as_str();
                $id_name(id, label)
            }
            /// Returns the [`TypeId`] used to differentiate labels.
            fn type_id(&self) -> ::core::any::TypeId {
                ::core::any::TypeId::of::<Self>()
            }
            /// Returns the representation of this label as a string literal.
            ///
            /// In cases where you absolutely need a label to be determined at runtime,
            /// you can use [`Box::leak`] to get a `'static` reference.
            fn as_str(&self) -> &'static str;
        }

        impl $label_name for $id_name {
            fn as_label(&self) -> Self {
                *self
            }
            fn type_id(&self) -> ::core::any::TypeId {
                self.0
            }
            fn as_str(&self) -> &'static str {
                self.1
            }
        }

        impl $label_name for &'static str {
            fn as_str(&self) -> Self {
                self
            }
        }
    };
}

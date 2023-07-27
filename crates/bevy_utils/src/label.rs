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
/// # use bevy_utils::define_label;
/// define_label!(
///     /// Documentation of label trait
///     MyNewLabelTrait,
///     MY_NEW_LABEL_TRAIT_INTERNER
/// );
/// ```
#[macro_export]
macro_rules! define_label {
    (
        $(#[$label_attr:meta])*
        $label_trait_name:ident,
        $interner_name:ident
    ) => {

        $(#[$label_attr])*
        pub trait $label_trait_name: 'static + Send + Sync + ::std::fmt::Debug {
            /// Return's the [TypeId] of this label, or the the ID of the
            /// wrappped label type for `Box<dyn
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

            /// Returns a static reference to a value equal to `self`, if possible.
            /// This method is used to optimize [interning](bevy_utils::intern).
            ///
            /// # Invariant
            ///
            /// The following invariants must be hold:
            ///
            /// `ptr_eq(a.dyn_static_ref(), b.dyn_static_ref())` if `a.dyn_eq(b)`
            /// `ptr_neq(a.dyn_static_ref(), b.dyn_static_ref())` if `!a.dyn_eq(b)`
            ///
            /// where `ptr_eq` and `ptr_neq` are defined as :
            /// ```
            /// fn ptr_eq<T>(x: Option<&'static T>, y: Option<&'static T>) -> bool {
            ///     match (x, y) {
            ///         (Some(x), Some(y)) => std::ptr::eq(x, y),
            ///         (None, None) => true,
            ///         _ => false,
            ///     }
            /// }
            ///
            /// fn ptr_neq<T>(x: Option<&'static T>, y: Option<&'static T>) -> bool {
            ///     match (x, y) {
            ///         (Some(x), Some(y)) => !std::ptr::eq(x, y),
            ///         (None, None) => true,
            ///         _ => false,
            ///     }
            /// }
            /// ```
            ///
            /// # Provided implementation
            ///
            /// The provided implementation always returns `None`.
            fn dyn_static_ref(&self) -> Option<&'static dyn $label_trait_name> {
                None
            }
        }

        impl $label_trait_name for ::bevy_utils::intern::Interned<dyn $label_trait_name> {
            fn inner_type_id(&self) -> ::std::any::TypeId {
                (**self).inner_type_id()
            }

            fn dyn_clone(&self) -> Box<dyn $label_trait_name> {
                (**self).dyn_clone()
            }

            /// Casts this value to a form where it can be compared with other type-erased values.
            fn as_dyn_eq(&self) -> &dyn ::bevy_utils::label::DynEq {
                (**self).as_dyn_eq()
            }

            fn dyn_hash(&self, state: &mut dyn ::std::hash::Hasher) {
                (**self).dyn_hash(state)
            }

            fn dyn_static_ref(&self) -> Option<&'static dyn $label_trait_name> {
                Some(self.0)
            }
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

        impl ::bevy_utils::intern::Leak for dyn $label_trait_name {
            fn leak(&self) -> &'static Self {
                Box::leak(self.dyn_clone())
            }

            fn static_ref(&self) -> std::option::Option<&'static dyn $label_trait_name> {
                self.dyn_static_ref()
            }
        }

        static $interner_name: ::bevy_utils::intern::Interner<dyn $label_trait_name> =
            ::bevy_utils::intern::Interner::new();

        impl From<&dyn $label_trait_name>
            for ::bevy_utils::intern::Interned<dyn $label_trait_name>
        {
            fn from(
                value: &dyn $label_trait_name,
            ) -> ::bevy_utils::intern::Interned<dyn $label_trait_name> {
                $interner_name.intern(value)
            }
        }
    };
}

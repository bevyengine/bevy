//! Traits used by label implementations

use std::{
    any::Any,
    hash::{Hash, Hasher},
};

pub trait DynEq: Any {
    fn as_any(&self) -> &dyn Any;

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

pub trait DynHash: DynEq {
    fn as_dyn_eq(&self) -> &dyn DynEq;

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
/// define_label!(MyNewLabelTrait);
/// ```
#[macro_export]
macro_rules! define_label {
    ($label_type_name:ident, $as_label:ident) => {
        /// Stores one of a set of strongly-typed labels for a class of objects.
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $label_type_name(::core::any::TypeId, &'static str);

        impl ::core::fmt::Debug for $label_type_name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                write!(f, "{}", self.1)
            }
        }

        /// Types that can be coerced into [`$label_type_name`].
        pub trait $as_label: 'static {
            /// Converts this type into an opaque, strongly-typed label.
            fn as_label(&self) -> $label_type_name {
                let id = self.type_id();
                let label = self.as_str();
                $label_type_name(id, label)
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

        impl $as_label for $label_type_name {
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

        impl $as_label for &'static str {
            fn as_str(&self) -> Self {
                self
            }
        }
    };
}

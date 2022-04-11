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
/// define_label!(MyNewLabelTrait, MyNewBoxedLabelTrait);
/// ```
#[macro_export]
macro_rules! define_label {
    ($label_trait_name:ident, $boxed_type_name:ident) => {
        /// Defines a set of strongly-typed labels for a class of objects
        pub trait $label_trait_name:
            ::bevy_utils::label::DynHash + ::std::fmt::Debug + Send + Sync + 'static
        {
            #[doc(hidden)]
            fn dyn_clone(&self) -> $boxed_type_name;
        }

        impl PartialEq for dyn $label_trait_name {
            fn eq(&self, other: &Self) -> bool {
                self.dyn_eq(other.as_dyn_eq())
            }
        }

        impl Eq for dyn $label_trait_name {}

        impl ::std::hash::Hash for dyn $label_trait_name {
            fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
                self.dyn_hash(state);
            }
        }

        impl Clone for $boxed_type_name {
            fn clone(&self) -> Self {
                self.dyn_clone()
            }
        }

        impl $label_trait_name for ::std::borrow::Cow<'static, str> {
            fn dyn_clone(&self) -> $boxed_type_name {
                $boxed_type_name(Box::new(self.clone()))
            }
        }

        impl $label_trait_name for &'static str {
            fn dyn_clone(&self) -> $boxed_type_name {
                $boxed_type_name(Box::new(<&str>::clone(self)))
            }
        }

        impl ::std::ops::Deref for $boxed_type_name {
            type Target = <Box<dyn $label_trait_name> as ::std::ops::Deref>::Target;

            fn deref(&self) -> &Self::Target {
                self.0.deref()
            }
        }

        /// A wrapper for a boxed label
        #[derive(Debug)]
        pub struct $boxed_type_name(Box<dyn $label_trait_name>);

        impl $boxed_type_name {
            #[doc(hidden)]
            pub fn new(label: Box<dyn $label_trait_name>) -> Self {
                Self(label)
            }
        }

        impl ::std::hash::Hash for $boxed_type_name {
            fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
                self.0.as_ref().dyn_hash(state);
            }
        }

        impl PartialEq for $boxed_type_name {
            fn eq(&self, other: &Self) -> bool {
                self.0.as_ref().dyn_eq(other.as_dyn_eq())
            }
        }

        impl Eq for $boxed_type_name {}

        impl $label_trait_name for $boxed_type_name {
            fn dyn_clone(&self) -> Self {
                self.0.as_ref().dyn_clone()
            }
        }
    };
}

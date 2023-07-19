//! Traits used by label implementations

use std::any::{Any, TypeId};
use std::collections::hash_map::DefaultHasher;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::sync::{OnceLock, RwLock};

use crate::{default, HashMap};

type Interner = RwLock<HashMap<UniqueValue, &'static str>>;
static INTERNER: OnceLock<Interner> = OnceLock::new();

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

/// The [`TypeId`](std::any::TypeId) of a value and its [`DynHash`] output
/// when hashed with the [`DefaultHasher`].
///
/// This is expected to have sufficient entropy to be different for each value.
#[derive(Clone, Copy, Eq, PartialEq, Hash)]
struct UniqueValue(TypeId, u64);

impl UniqueValue {
    fn of<T: DynHash + ?Sized>(value: &T) -> Self {
        let mut hasher = DefaultHasher::new();
        value.dyn_hash(&mut hasher);
        let hash = hasher.finish();

        Self(TypeId::of::<T>(), hash)
    }
}

/// Returns a reference to the interned `Debug` string of `value`.
///
/// If the string has not been interned, it will be allocated in a [`Box`] and leaked, but
/// subsequent calls with the same `value` will return the same reference.
pub fn intern_debug_string<T>(value: &T) -> &'static str
where
    T: Debug + DynHash + ?Sized,
{
    let key = UniqueValue::of(value);
    let mut map = INTERNER.get_or_init(default).write().unwrap();
    let str = *map.entry(key).or_insert_with(|| {
        let string = format!("{:?}", value);
        let str: &'static str = Box::leak(string.into_boxed_str());
        str
    });

    str
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
        $(#[$label_attr])*
        pub trait $label_name: bevy_utils::label::DynHash + ::std::fmt::Debug + Send + Sync + 'static {
            /// Returns the unique, type-elided identifier for `self`.
            fn as_label(&self) -> $id_name {
                $id_name::of(self)
            }
        }

        $(#[$id_attr])*
        #[derive(Clone, Copy, Eq)]
        pub struct $id_name(&'static str);

        impl $id_name {
            /// Returns the [`
            #[doc = stringify!($id_name)]
            ///`] of the [`
            #[doc = stringify!($label_name)]
            ///`].
            pub fn of<T>(label: &T) -> $id_name
            where
                T: $label_name + ?Sized,
            {
                $id_name(bevy_utils::label::intern_debug_string(label))
            }
        }

        impl ::std::cmp::PartialEq for $id_name {
            fn eq(&self, other: &Self) -> bool {
                ::std::ptr::eq(self.0, other.0)
            }
        }

        impl ::std::hash::Hash for $id_name {
            fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
                ::std::ptr::hash(self.0, state);
            }
        }

        impl ::std::fmt::Debug for $id_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl $label_name for $id_name {
            fn as_label(&self) -> Self {
                *self
            }
        }

        impl ::std::convert::AsRef<dyn $label_name> for $id_name {
            #[inline]
            fn as_ref(&self) -> &dyn $label_name {
                self
            }
        }

        impl ::std::convert::AsRef<dyn $label_name> for dyn $label_name {
            #[inline]
            fn as_ref(&self) -> &dyn $label_name {
                self
            }
        }
    };
}

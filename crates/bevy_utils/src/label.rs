//! Traits used by label implementations

use std::{
    any::Any,
    hash::{Hash, Hasher},
};

use crate::Interner;

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

#[doc(hidden)]
pub struct VTable {
    // FIXME: When const TypeId stabilizes, inline the type instead of using a fn pointer for indirection.
    pub ty: fn() -> ::std::any::TypeId,
    pub fmt: fn(u64, &mut ::std::fmt::Formatter) -> ::std::fmt::Result,
}

impl PartialEq for VTable {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        (self.ty)() == (other.ty)()
    }
}
impl Eq for VTable {}

impl Hash for VTable {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.ty)().hash(state);
    }
}

#[doc(hidden)]
pub static STR_INTERN: Interner<&str> = Interner::new();

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
        pub struct $id_name {
            data: u64,
            vtable: &'static $crate::label::VTable,
        }

        impl ::std::fmt::Debug for $id_name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                (self.vtable.fmt)(self.data, f)
            }
        }

        $(#[$label_attr])*
        pub trait $label_name: 'static {
            /// Converts this type into an opaque, strongly-typed label.
            #[inline]
            fn as_label(&self) -> $id_name {
                // This is just machinery that lets us store the TypeId and formatter fn in the same static reference.
                struct VTables<L: ?::std::marker::Sized>(L);
                impl<L: $label_name + ?::std::marker::Sized> VTables<L> {
                    const VTABLE: $crate::label::VTable = $crate::label::VTable {
                        ty: || ::std::any::TypeId::of::<L>(),
                        fmt: <L as $label_name>::fmt,
                    };
                }

                let data = self.data();
                $id_name { data, vtable: &VTables::<Self>::VTABLE }
            }
            /// Returns a number used to distinguish different labels of the same type.
            fn data(&self) -> u64;
            /// Writes debug info for a label of the current type.
            /// * `data`: the result of calling [`data()`](#method.data) on an instance of this type.
            ///
            /// You should not call this method directly, as it may panic for some types;
            /// use [`as_label`](#method.as_label) instead.
            fn fmt(data: u64, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result;
        }

        impl $label_name for $id_name {
            #[inline]
            fn as_label(&self) -> Self {
                *self
            }
            #[inline]
            fn data(&self) -> u64 {
                self.data
            }
            #[track_caller]
            fn fmt(data: u64, f: &mut ::std::fmt::Formatter) -> std::fmt::Result {
                let label = stringify!($label_name);
                ::std::unimplemented!("do not call `{label}::fmt` directly -- use the result of `as_label()` for formatting instead")
            }
        }

        impl $id_name {
            /// Returns the [`TypeId`] of the label from which this ID was constructed.
            ///
            /// [`TypeId`]: ::std::any::TypeId
            #[inline]
            pub fn type_id(self) -> ::std::any::TypeId {
                (self.vtable.ty)()
            }
            /// Returns true if this label was constructed from an instance of type `L`.
            pub fn is<L: $label_name>(self) -> bool {
                self.type_id() == ::std::any::TypeId::of::<L>()
            }
        }

        impl $label_name for &'static str {
            fn data(&self) -> u64 {
                $crate::label::STR_INTERN.intern(self) as u64
            }
            fn fmt(idx: u64, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                let s = $crate::label::STR_INTERN
                    .get(idx as usize)
                    .ok_or(::std::fmt::Error)?;
                write!(f, "{s}")
            }
        }
    };
}

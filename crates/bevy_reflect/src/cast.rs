//! Traits for casting types to [`dyn PartialReflect`] and [`dyn Reflect`] trait objects.
//!
//! These traits are used internally by the [derive macro] and other [`Reflect`] implementations,
//! to allow for transparent wrapper types, such as [`Box`], to be used as fields.
//!
//! # Custom Trait Objects
//!
//! These traits also enable the usage of custom trait objects as reflected fields.
//!
//! The only requirements are:
//! - The trait must have at least [`CastPartialReflect`] as a supertrait.
//!   This includes using [`CastReflect`], [`PartialReflect`], or [`Reflect`] as supertraits,
//!   since they are all subtraits of [`CastPartialReflect`].
//! - The trait must implement [`TypePath`] for its trait object
//!
//! ```
//! # use bevy_reflect::{PartialReflect, Reflect, Struct, TypePath};
//! #
//! trait Equippable: PartialReflect {}
//!
//! impl TypePath for dyn Equippable {
//!     fn type_path() -> &'static str {
//!         "dyn my_crate::Equippable"
//!     }
//!
//!     fn short_type_path() -> &'static str {
//!         "dyn Equippable"
//!     }
//! }
//!
//! #[derive(Reflect)]
//! struct Sword(u32);
//!
//! impl Equippable for Sword {}
//!
//! #[derive(Reflect)]
//! #[reflect(from_reflect = false)]
//! struct Player {
//!    weapon: Box<dyn Equippable>,
//! }
//!
//! let player: Box<dyn Struct> = Box::new(Player {
//!     weapon: Box::new(Sword(123)),
//! });
//!
//! let weapon = player.field("weapon").unwrap();
//! assert!(weapon.reflect_partial_eq(&Sword(123)).unwrap_or_default());
//! ```
//!
//! [`dyn PartialReflect`]: PartialReflect
//! [`dyn Reflect`]: crate::Reflect
//! [derive macro]: derive@crate::Reflect
//! [`Reflect`]: crate::Reflect
//! [`TypePath`]: crate::TypePath

use crate::utility::GenericTypeInfoCell;
use crate::{
    GetTypeRegistration, OpaqueInfo, PartialReflect, Reflect, TypeInfo, TypePath, TypeRegistration,
    Typed,
};
use alloc::boxed::Box;
use bevy_reflect_derive::impl_type_path;

/// A trait used to cast `Self` to a [`dyn PartialReflect`] trait object.
///
/// This is automatically implemented for any type that [derives `Reflect`],
/// as well as [`Box<T>`] where `T` also implements [`CastPartialReflect`].
///
/// [`dyn PartialReflect`]: PartialReflect
/// [derives `Reflect`]: derive@crate::Reflect
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not implement `CastPartialReflect` so cannot be cast to `dyn PartialReflect`",
    note = "consider annotating `{Self}` with `#[derive(Reflect)]`"
)]
pub trait CastPartialReflect: Send + Sync + 'static {
    /// Casts this type to a [`dyn PartialReflect`] reference.
    ///
    /// This is useful for coercing trait objects.
    ///
    /// [`dyn PartialReflect`]: PartialReflect
    fn as_partial_reflect(&self) -> &dyn PartialReflect;

    /// Casts this type to a mutable [`dyn PartialReflect`] reference.
    ///
    /// This is useful for coercing trait objects.
    ///
    /// [`dyn PartialReflect`]: PartialReflect
    fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect;

    /// Casts this type into a boxed [`dyn PartialReflect`] instance.
    ///
    /// This is useful for coercing trait objects.
    ///
    /// [`dyn PartialReflect`]: PartialReflect
    fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect>;
}

impl<T: ?Sized + CastPartialReflect> CastPartialReflect for Box<T> {
    fn as_partial_reflect(&self) -> &dyn PartialReflect {
        T::as_partial_reflect(self)
    }

    fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
        T::as_partial_reflect_mut(self)
    }

    fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect> {
        T::into_partial_reflect(*self)
    }
}

/// A trait used to cast `Self` to a [`dyn Reflect`] trait object.
///
/// This is automatically implemented for any type that [derives `Reflect`],
/// as well as [`Box<T>`] where `T` also implements [`CastReflect`].
///
/// [`dyn Reflect`]: Reflect
/// [derives `Reflect`]: derive@crate::Reflect
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not implement `CastReflect` so cannot be cast to `dyn Reflect`",
    note = "consider annotating `{Self}` with `#[derive(Reflect)]`"
)]
pub trait CastReflect: CastPartialReflect {
    /// Casts this type to a [`dyn Reflect`] reference.
    ///
    /// This is useful for coercing trait objects.
    ///
    /// [`dyn Reflect`]: Reflect
    fn as_reflect(&self) -> &dyn Reflect;

    /// Casts this type to a mutable [`dyn Reflect`] reference.
    ///
    /// This is useful for coercing trait objects.
    ///
    /// [`dyn Reflect`]: Reflect
    fn as_reflect_mut(&mut self) -> &mut dyn Reflect;

    /// Casts this type into a boxed [`dyn Reflect`] instance.
    ///
    /// This is useful for coercing trait objects.
    ///
    /// [`dyn Reflect`]: Reflect
    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect>;
}

impl<T: ?Sized + CastReflect> CastReflect for Box<T> {
    fn as_reflect(&self) -> &dyn Reflect {
        T::as_reflect(self)
    }

    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        T::as_reflect_mut(self)
    }

    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> {
        T::into_reflect(*self)
    }
}

impl_type_path!(::alloc::boxed::Box<T: ?Sized>);

impl<T: ?Sized + TypePath + Send + Sync> Typed for Box<T> {
    fn type_info() -> &'static TypeInfo {
        static CELL: GenericTypeInfoCell = GenericTypeInfoCell::new();
        CELL.get_or_insert::<Self, _>(|| TypeInfo::Opaque(OpaqueInfo::new::<Self>()))
    }
}

impl<T: ?Sized + TypePath + Send + Sync> GetTypeRegistration for Box<T> {
    fn get_type_registration() -> TypeRegistration {
        TypeRegistration::of::<Self>()
    }
}

macro_rules! impl_cast_partial_reflect {
    ($(<$($id:ident),* $(,)?>)? for $ty:ty $(where $($tt:tt)*)?) => {
        impl $(<$($id),*>)? $crate::cast::CastPartialReflect for $ty $(where $($tt)*)? {
            #[inline]
            fn as_partial_reflect(&self) -> &dyn $crate::PartialReflect {
                self
            }

            #[inline]
            fn as_partial_reflect_mut(&mut self) -> &mut dyn $crate::PartialReflect {
                self
            }

            #[inline]
            fn into_partial_reflect(self: Box<Self>) -> Box<dyn $crate::PartialReflect> {
                self
            }
        }
    };
}

pub(crate) use impl_cast_partial_reflect;

macro_rules! impl_casting_traits {
    ($(<$($id:ident),* $(,)?>)? for $ty:ty $(where $($tt:tt)*)?) => {

        $crate::cast::impl_cast_partial_reflect!($(<$($id),*>)? for $ty $(where $($tt)*)?);

        impl $(<$($id),*>)? $crate::cast::CastReflect for $ty $(where $($tt)*)? {
            #[inline]
            fn as_reflect(&self) -> &dyn $crate::Reflect {
                self
            }

            #[inline]
            fn as_reflect_mut(&mut self) -> &mut dyn $crate::Reflect {
                self
            }

            #[inline]
            fn into_reflect(self: Box<Self>) -> Box<dyn $crate::Reflect> {
                self
            }
        }
    };
}

pub(crate) use impl_casting_traits;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Struct, Tuple, TupleStruct};
    use static_assertions::assert_not_impl_all;

    #[test]
    fn should_not_reflect_box() {
        assert_not_impl_all!(Box<i32>: Reflect, PartialReflect);
        assert_not_impl_all!(Box<dyn PartialReflect>: Reflect, PartialReflect);
        assert_not_impl_all!(Box<dyn Reflect>: Reflect, PartialReflect);
    }

    #[test]
    fn should_reflect_boxed_struct_field() {
        #[derive(Reflect)]
        #[reflect(from_reflect = false)]
        struct MyStruct {
            value: Box<dyn Reflect>,
        }

        let my_struct: Box<dyn Struct> = Box::new(MyStruct {
            value: Box::new(123_i32),
        });

        let field = my_struct.field("value").unwrap();
        assert_eq!(field.try_downcast_ref::<i32>(), Some(&123));

        let field_info = field.get_represented_type_info().unwrap();
        assert!(field_info.ty().is::<i32>());
    }

    #[test]
    fn should_reflect_boxed_tuple_struct_field() {
        #[derive(Reflect)]
        #[reflect(from_reflect = false)]
        struct MyStruct(Box<dyn Reflect>);

        let my_struct: Box<dyn TupleStruct> = Box::new(MyStruct(Box::new(123_i32)));

        let field = my_struct.field(0).unwrap();
        assert_eq!(field.try_downcast_ref::<i32>(), Some(&123));

        let field_info = field.get_represented_type_info().unwrap();
        assert!(field_info.ty().is::<i32>());
    }

    #[test]
    fn should_reflect_boxed_tuple_field() {
        let my_struct: Box<dyn Tuple> = Box::new((Box::new(10_i32),));

        let field = my_struct.field(0).unwrap();
        assert_eq!(field.try_downcast_ref::<i32>(), Some(&10));

        let field_info = field.get_represented_type_info().unwrap();
        assert!(field_info.ty().is::<i32>());
    }

    #[test]
    fn should_allow_boxed_type_parameter() {
        #[derive(Reflect)]
        #[reflect(from_reflect = false)]
        struct MyStruct<T> {
            value: T,
        }

        let my_struct: Box<dyn Struct> = Box::new(MyStruct {
            value: Box::new(123_i32),
        });

        let field = my_struct.field("value").unwrap();
        assert_eq!(field.try_downcast_ref::<i32>(), Some(&123));

        let field_info = field.get_represented_type_info().unwrap();
        assert!(field_info.ty().is::<i32>());
    }

    #[test]
    fn should_allow_custom_trait_objects() {
        trait Equippable: CastPartialReflect {}

        impl TypePath for dyn Equippable {
            fn type_path() -> &'static str {
                "dyn my_crate::Equippable"
            }

            fn short_type_path() -> &'static str {
                "dyn Equippable"
            }
        }

        #[derive(Reflect)]
        struct Sword(u32);

        impl Equippable for Sword {}

        #[derive(Reflect)]
        #[reflect(from_reflect = false)]
        struct Player {
            weapon: Box<dyn Equippable>,
        }

        let player: Box<dyn Struct> = Box::new(Player {
            weapon: Box::new(Sword(123)),
        });

        let weapon = player.field("weapon").unwrap();
        assert!(weapon.reflect_partial_eq(&Sword(123)).unwrap_or_default());
    }
}

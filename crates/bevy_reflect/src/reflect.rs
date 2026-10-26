use crate::{
    array::array_debug,
    enums::{enum_debug, VariantType},
    list::list_debug,
    map::map_debug,
    set::set_debug,
    structs::struct_debug,
    tuple::tuple_debug,
    tuple_struct::tuple_struct_debug,
    DynamicTypePath, DynamicTyped, OpaqueInfo, ReflectCloneError, ReflectKind,
    ReflectKindMismatchError, ReflectMut, ReflectOwned, ReflectRef, TypeInfo, TypePath, Typed,
};
use alloc::borrow::Cow;
use alloc::boxed::Box;
use alloc::string::ToString;
use core::{
    any::{Any, TypeId},
    cmp::Ordering,
    fmt::Debug,
};

use thiserror::Error;

use crate::utility::NonGenericTypeInfoCell;

/// A enumeration of all error outcomes that might happen when running [`try_apply`](Reflect::try_apply).
#[derive(Error, Debug)]
pub enum ApplyError {
    #[error("attempted to apply `{from_kind}` to `{to_kind}`")]
    /// Attempted to apply the wrong [kind](ReflectKind) to a type, e.g. a struct to an enum.
    MismatchedKinds {
        /// Kind of the value we attempted to apply.
        from_kind: ReflectKind,
        /// Kind of the type we attempted to apply the value to.
        to_kind: ReflectKind,
    },

    #[error("enum variant `{variant_name}` was unable to be converted to an enum value through `FromDynamic`")]
    /// Enum variant that we tried to apply could not be converted using the enum's [`FromDynamic`].
    ///
    /// [`FromDynamic`]: crate::FromDynamic
    InvalidEnumVariant {
        /// Name of the enum variant.
        variant_name: Box<str>,
    },

    #[error("attempted to apply enum variant type {from_type:?} to {to_type:?}")]
    /// Attempted to apply the wrong enum [`VariantType`].
    WrongEnumVariantType {
        /// Type of the enum variant we attempted to apply.
        from_type: VariantType,
        /// Type of the enum variant we attempted to apply the value to.
        to_type: VariantType,
    },

    #[error("failed to convert dynamic value into concrete value for list/map/set")]
    /// Failed to construct a concrete value for a list/map/set from its dynamic representation.
    InvalidDynamicValueForCollection,

    #[error("`{from_type}` is not `{to_type}`")]
    /// Tried to apply incompatible types.
    MismatchedTypes {
        /// Type of the value we attempted to apply.
        from_type: Box<str>,
        /// Type we attempted to apply the value to.
        to_type: Box<str>,
    },

    #[error("attempted to apply type with {from_size} size to a type with {to_size} size")]
    /// Attempted to apply an [array-like] type to another of different size, e.g. a [u8; 4] to [u8; 3].
    ///
    /// [array-like]: crate::array::Array
    DifferentSize {
        /// Size of the value we attempted to apply, in elements.
        from_size: usize,
        /// Size of the type we attempted to apply the value to, in elements.
        to_size: usize,
    },

    #[error("variant with name `{variant_name}` does not exist on enum `{enum_name}`")]
    /// The enum we tried to apply to didn't contain a variant with the give name.
    UnknownVariant {
        /// Name of the enum.
        enum_name: Box<str>,
        /// Name of the missing variant.
        variant_name: Box<str>,
    },

    #[error(transparent)]
    /// A value could not be converted to its dynamic representation via
    /// [`Reflect::to_dynamic`] while applying it.
    CloneError(#[from] ReflectCloneError),
}

impl From<ReflectKindMismatchError> for ApplyError {
    fn from(value: ReflectKindMismatchError) -> Self {
        Self::MismatchedKinds {
            from_kind: value.expected,
            to_kind: value.received,
        }
    }
}

/// A core trait of [`bevy_reflect`], used for downcasting to concrete types.
///
/// This is a subtrait of [`Reflect`],
/// meaning any type which implements `Reflect` implements `Reflect` by definition.
///
/// It's recommended to use [the derive macro] rather than manually implementing this trait.
/// Doing so will automatically implement this trait, [`Reflect`], and many other useful traits for reflection,
/// including one of the appropriate subtraits: [`Struct`], [`TupleStruct`] or [`Enum`].
///
/// If you need to use this trait as a generic bound along with other reflection traits,
/// for your convenience, consider using [`Reflectable`] instead.
///
/// See the [crate-level documentation] to see how this trait can be used.
///
/// [`bevy_reflect`]: crate
/// [the derive macro]: bevy_reflect_derive::Reflect
/// [`Struct`]: crate::structs::Struct
/// [`TupleStruct`]: crate::tuple_struct::TupleStruct
/// [`Enum`]: crate::enums::Enum
/// [`Reflectable`]: crate::Reflectable
/// [crate-level documentation]: crate
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not implement `Reflect` so cannot be fully reflected",
    note = "consider annotating `{Self}` with `#[derive(Reflect)]`"
)]
pub trait Reflect: DynamicTyped + DynamicTypePath + Send + Sync + Any {
    /// Returns a zero-sized enumeration of "kinds" of type.
    ///
    /// See [`ReflectKind`].
    fn reflect_kind(&self) -> ReflectKind {
        self.reflect_ref().kind()
    }

    /// Returns an immutable enumeration of "kinds" of type.
    ///
    /// See [`ReflectRef`].
    fn reflect_ref(&self) -> ReflectRef<'_>;

    /// Returns a mutable enumeration of "kinds" of type.
    ///
    /// See [`ReflectMut`].
    fn reflect_mut(&mut self) -> ReflectMut<'_>;

    /// Returns an owned enumeration of "kinds" of type.
    ///
    /// See [`ReflectOwned`].
    fn reflect_owned(self: Box<Self>) -> ReflectOwned;

    /// Attempts to clone `Self` using reflection.
    ///
    /// Unlike [`to_dynamic`], which generally returns a dynamic representation of `Self`,
    /// this method attempts create a clone of `Self` directly, if possible.
    ///
    /// If the clone cannot be performed, an appropriate [`ReflectCloneError`] is returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::Reflect;
    /// let value = (1, true, 3.14);
    /// let cloned = value.reflect_clone().unwrap();
    /// assert!(cloned.is::<(i32, bool, f64)>())
    /// ```
    ///
    /// [`to_dynamic`]: Reflect::to_dynamic
    fn reflect_clone(&self) -> Result<Box<dyn Reflect>, ReflectCloneError> {
        Err(ReflectCloneError::NotImplemented {
            type_path: Cow::Owned(self.reflect_type_path().to_string()),
        })
    }

    /// For a type implementing [`Reflect`], combines `reflect_clone` and
    /// `take` in a useful fashion, automatically constructing an appropriate
    /// [`ReflectCloneError`] if the downcast fails.
    fn reflect_clone_and_take<T: 'static>(&self) -> Result<T, ReflectCloneError>
    where
        Self: TypePath + Sized,
    {
        self.reflect_clone()?
            .take()
            .map_err(|_| ReflectCloneError::FailedDowncast {
                expected: Cow::Borrowed(<Self as TypePath>::type_path()),
                received: Cow::Owned(self.reflect_type_path().to_string()),
            })
    }

    /// Returns a hash of the value (which includes the type).
    ///
    /// If the underlying type does not support hashing, returns `None`.
    fn reflect_hash(&self) -> Option<u64> {
        None
    }

    /// Returns a "partial equality" comparison result.
    ///
    /// If the underlying type does not support equality testing, returns `None`.
    fn reflect_partial_eq(&self, _value: &dyn Reflect) -> Option<bool> {
        None
    }

    /// Returns a "partial comparison" result.
    ///
    /// If the underlying type does not support it, returns `None`.
    fn reflect_partial_cmp(&self, _value: &dyn Reflect) -> Option<Ordering> {
        None
    }

    /// Debug formatter for the value.
    ///
    /// Any value that is not an implementor of other `Reflect` subtraits
    /// (e.g. [`List`], [`Map`]), will default to the format: `"Reflect(type_path)"`,
    /// where `type_path` is the [type path] of the underlying type.
    ///
    /// [`List`]: crate::list::List
    /// [`Map`]: crate::map::Map
    /// [type path]: TypePath::type_path
    fn debug(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.reflect_ref() {
            ReflectRef::Struct(dyn_struct) => struct_debug(dyn_struct, f),
            ReflectRef::TupleStruct(dyn_tuple_struct) => tuple_struct_debug(dyn_tuple_struct, f),
            ReflectRef::Tuple(dyn_tuple) => tuple_debug(dyn_tuple, f),
            ReflectRef::List(dyn_list) => list_debug(dyn_list, f),
            ReflectRef::Array(dyn_array) => array_debug(dyn_array, f),
            ReflectRef::Map(dyn_map) => map_debug(dyn_map, f),
            ReflectRef::Set(dyn_set) => set_debug(dyn_set, f),
            ReflectRef::Enum(dyn_enum) => enum_debug(dyn_enum, f),
            ReflectRef::Opaque(_) => write!(f, "Reflect({})", self.reflect_type_path()),
        }
    }

    /// Returns the value as a [`Box<dyn Any>`][core::any::Any].
    ///
    /// For remote wrapper types, this will return the remote type instead.
    fn into_any(self: Box<Self>) -> Box<dyn Any>;

    /// Returns the value as a [`&dyn Any`][core::any::Any].
    ///
    /// For remote wrapper types, this will return the remote type instead.
    fn as_any(&self) -> &dyn Any;

    /// Returns the value as a [`&mut dyn Any`][core::any::Any].
    ///
    /// For remote wrapper types, this will return the remote type instead.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Casts this type to a boxed, fully-reflected value.
    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect>;

    /// Casts this type to a fully-reflected value.
    fn as_reflect(&self) -> &dyn Reflect;

    /// Casts this type to a mutable, fully-reflected value.
    fn as_reflect_mut(&mut self) -> &mut dyn Reflect;

    /// Performs a type-checked assignment of a reflected value to this value.
    ///
    /// If `value` does not contain a value of type `T`, returns an `Err`
    /// containing the trait object.
    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>>;
}

#[deny(rustdoc::broken_intra_doc_links)]
impl dyn Reflect {
    /// Downcasts the value to type `T`, consuming the trait object.
    ///
    /// If the underlying value is not of type `T`, returns `Err(self)`.
    ///
    /// For remote types, `T` should be the type itself rather than the wrapper type.
    pub fn downcast<T: Any>(self: Box<dyn Reflect>) -> Result<Box<T>, Box<dyn Reflect>> {
        if self.is::<T>() {
            Ok(self.into_any().downcast().unwrap())
        } else {
            Err(self)
        }
    }

    /// Downcasts the value to type `T`, unboxing and consuming the trait object.
    ///
    /// If the underlying value is not of type `T`, returns `Err(self)`.
    ///
    /// For remote types, `T` should be the type itself rather than the wrapper type.
    pub fn take<T: Any>(self: Box<dyn Reflect>) -> Result<T, Box<dyn Reflect>> {
        self.downcast::<T>().map(|value| *value)
    }

    /// Returns `true` if the underlying value is of type `T`, or `false`
    /// otherwise.
    ///
    /// The underlying value is the concrete type that is stored in this `dyn` object;
    /// it can be downcast to. In the case that this underlying value "represents"
    /// a different type, like the Dynamic\*\*\* types do, you can call `represents`
    /// to determine what type they represent. Represented types cannot be downcast
    /// to, but you can use [`FromDynamic`] to create a value of the represented type from them.
    ///
    /// For remote types, `T` should be the type itself rather than the wrapper type.
    ///
    /// [`FromDynamic`]: crate::FromDynamic
    #[inline]
    pub fn is<T: Any>(&self) -> bool {
        self.as_any().type_id() == TypeId::of::<T>()
    }

    /// Downcasts the value to type `T` by reference.
    ///
    /// If the underlying value is not of type `T`, returns `None`.
    ///
    /// For remote types, `T` should be the type itself rather than the wrapper type.
    #[inline]
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.as_any().downcast_ref::<T>()
    }

    /// Downcasts the value to type `T` by mutable reference.
    ///
    /// If the underlying value is not of type `T`, returns `None`.
    ///
    /// For remote types, `T` should be the type itself rather than the wrapper type.
    #[inline]
    pub fn downcast_mut<T: Any>(&mut self) -> Option<&mut T> {
        self.as_any_mut().downcast_mut::<T>()
    }
}

impl Debug for dyn Reflect {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.debug(f)
    }
}

impl Typed for dyn Reflect {
    fn type_info() -> &'static TypeInfo {
        static CELL: NonGenericTypeInfoCell = NonGenericTypeInfoCell::new();
        CELL.get_or_set(|| TypeInfo::Opaque(OpaqueInfo::new::<Self>()))
    }
}

// The following implementation never actually shadows the concrete `TypePath` implementation.
// See this playground (https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=589064053f27bc100d90da89c6a860aa).
impl TypePath for dyn Reflect {
    fn type_path() -> &'static str {
        "dyn bevy_reflect::Reflect"
    }

    fn short_type_path() -> &'static str {
        "dyn Reflect"
    }
}

macro_rules! impl_reflect_casts {
    () => {
        fn into_any(
            self: bevy_platform::prelude::Box<Self>,
        ) -> bevy_platform::prelude::Box<dyn ::core::any::Any> {
            self
        }

        fn as_any(&self) -> &dyn ::core::any::Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn ::core::any::Any {
            self
        }

        fn into_reflect(
            self: bevy_platform::prelude::Box<Self>,
        ) -> bevy_platform::prelude::Box<dyn $crate::Reflect> {
            self
        }

        fn as_reflect(&self) -> &dyn $crate::Reflect {
            self
        }

        fn as_reflect_mut(&mut self) -> &mut dyn $crate::Reflect {
            self
        }

        fn set(
            &mut self,
            value: bevy_platform::prelude::Box<dyn $crate::Reflect>,
        ) -> Result<(), bevy_platform::prelude::Box<dyn $crate::Reflect>> {
            *self = <dyn $crate::Reflect>::take(value)?;
            Ok(())
        }
    };
}

pub(crate) use impl_reflect_casts;

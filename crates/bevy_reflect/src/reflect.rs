use crate::{
    array_debug, enum_debug, list_debug, map_debug, set_debug, struct_debug, tuple_debug,
    tuple_struct_debug, DynamicTypePath, DynamicTyped, OpaqueInfo, ReflectCloneError, ReflectKind,
    ReflectKindMismatchError, ReflectMut, ReflectOwned, ReflectRef, TypeInfo, TypePath, Typed,
};
use alloc::borrow::Cow;
use alloc::boxed::Box;
use alloc::string::ToString;
use core::{
    any::{Any, TypeId},
    fmt::Debug,
};

use thiserror::Error;

use crate::utility::NonGenericTypeInfoCell;

/// A enumeration of all error outcomes that might happen when running [`try_apply`](PartialReflect::try_apply).
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

    #[error("enum variant `{variant_name}` doesn't have a field named `{field_name}`")]
    /// Enum variant that we tried to apply to was missing a field.
    MissingEnumField {
        /// Name of the enum variant.
        variant_name: Box<str>,
        /// Name of the missing field.
        field_name: Box<str>,
    },

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
    /// [array-like]: crate::Array
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
}

impl From<ReflectKindMismatchError> for ApplyError {
    fn from(value: ReflectKindMismatchError) -> Self {
        Self::MismatchedKinds {
            from_kind: value.received,
            to_kind: value.expected,
        }
    }
}

/// The foundational trait of [`bevy_reflect`], used for accessing and modifying data dynamically.
///
/// This is a supertrait of [`Reflect`],
/// meaning any type which implements `Reflect` implements `PartialReflect` by definition.
///
/// It's recommended to use [the derive macro for `Reflect`] rather than manually implementing this trait.
/// Doing so will automatically implement this trait as well as many other useful traits for reflection,
/// including one of the appropriate subtraits: [`Struct`], [`TupleStruct`] or [`Enum`].
///
/// See the [crate-level documentation] to see how this trait and its subtraits can be used.
///
/// [`bevy_reflect`]: crate
/// [the derive macro for `Reflect`]: bevy_reflect_derive::Reflect
/// [`Struct`]: crate::Struct
/// [`TupleStruct`]: crate::TupleStruct
/// [`Enum`]: crate::Enum
/// [crate-level documentation]: crate
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not implement `PartialReflect` so cannot be introspected",
    note = "consider annotating `{Self}` with `#[derive(Reflect)]`"
)]
pub trait PartialReflect: DynamicTypePath + Send + Sync
where
    // NB: we don't use `Self: Any` since for downcasting, `Reflect` should be used.
    Self: 'static,
{
    /// Returns the [`TypeInfo`] of the type _represented_ by this value.
    ///
    /// For most types, this will simply return their own `TypeInfo`.
    /// However, for dynamic types, such as [`DynamicStruct`] or [`DynamicList`],
    /// this will return the type they represent
    /// (or `None` if they don't represent any particular type).
    ///
    /// This method is great if you have an instance of a type or a `dyn Reflect`,
    /// and want to access its [`TypeInfo`]. However, if this method is to be called
    /// frequently, consider using [`TypeRegistry::get_type_info`] as it can be more
    /// performant for such use cases.
    ///
    /// [`DynamicStruct`]: crate::DynamicStruct
    /// [`DynamicList`]: crate::DynamicList
    /// [`TypeRegistry::get_type_info`]: crate::TypeRegistry::get_type_info
    fn get_represented_type_info(&self) -> Option<&'static TypeInfo>;

    /// Casts this type to a boxed, reflected value.
    ///
    /// This is useful for coercing trait objects.
    fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect>;

    /// Casts this type to a reflected value.
    ///
    /// This is useful for coercing trait objects.
    fn as_partial_reflect(&self) -> &dyn PartialReflect;

    /// Casts this type to a mutable, reflected value.
    ///
    /// This is useful for coercing trait objects.
    fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect;

    /// Attempts to cast this type to a boxed, [fully-reflected] value.
    ///
    /// [fully-reflected]: Reflect
    fn try_into_reflect(self: Box<Self>) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>>;

    /// Attempts to cast this type to a [fully-reflected] value.
    ///
    /// [fully-reflected]: Reflect
    fn try_as_reflect(&self) -> Option<&dyn Reflect>;

    /// Attempts to cast this type to a mutable, [fully-reflected] value.
    ///
    /// [fully-reflected]: Reflect
    fn try_as_reflect_mut(&mut self) -> Option<&mut dyn Reflect>;

    /// Applies a reflected value to this value.
    ///
    /// If `Self` implements a [reflection subtrait], then the semantics of this
    /// method are as follows:
    /// - If `Self` is a [`Struct`], then the value of each named field of `value` is
    ///   applied to the corresponding named field of `self`. Fields which are
    ///   not present in both structs are ignored.
    /// - If `Self` is a [`TupleStruct`] or [`Tuple`], then the value of each
    ///   numbered field is applied to the corresponding numbered field of
    ///   `self.` Fields which are not present in both values are ignored.
    /// - If `Self` is an [`Enum`], then the variant of `self` is `updated` to match
    ///   the variant of `value`. The corresponding fields of that variant are
    ///   applied from `value` onto `self`. Fields which are not present in both
    ///   values are ignored.
    /// - If `Self` is a [`List`] or [`Array`], then each element of `value` is applied
    ///   to the corresponding element of `self`. Up to `self.len()` items are applied,
    ///   and excess elements in `value` are appended to `self`.
    /// - If `Self` is a [`Map`], then for each key in `value`, the associated
    ///   value is applied to the value associated with the same key in `self`.
    ///   Keys which are not present in `self` are inserted, and keys from `self` which are not present in `value` are removed.
    /// - If `Self` is a [`Set`], then each element of `value` is applied to the corresponding
    ///   element of `Self`. If an element of `value` does not exist in `Self` then it is
    ///   cloned and inserted. If an element from `self` is not present in `value` then it is removed.
    /// - If `Self` is none of these, then `value` is downcast to `Self`, cloned, and
    ///   assigned to `self`.
    ///
    /// Note that `Reflect` must be implemented manually for [`List`]s,
    /// [`Map`]s, and [`Set`]s in order to achieve the correct semantics, as derived
    /// implementations will have the semantics for [`Struct`], [`TupleStruct`], [`Enum`]
    /// or none of the above depending on the kind of type. For lists, maps, and sets, use the
    /// [`list_apply`], [`map_apply`], and [`set_apply`] helper functions when implementing this method.
    ///
    /// [reflection subtrait]: crate#the-reflection-subtraits
    /// [`Struct`]: crate::Struct
    /// [`TupleStruct`]: crate::TupleStruct
    /// [`Tuple`]: crate::Tuple
    /// [`Enum`]: crate::Enum
    /// [`List`]: crate::List
    /// [`Array`]: crate::Array
    /// [`Map`]: crate::Map
    /// [`Set`]: crate::Set
    /// [`list_apply`]: crate::list_apply
    /// [`map_apply`]: crate::map_apply
    /// [`set_apply`]: crate::set_apply
    ///
    /// # Panics
    ///
    /// Derived implementations of this method will panic:
    /// - If the type of `value` is not of the same kind as `Self` (e.g. if `Self` is
    ///   a `List`, while `value` is a `Struct`).
    /// - If `Self` is any complex type and the corresponding fields or elements of
    ///   `self` and `value` are not of the same type.
    /// - If `Self` is an opaque type and `value` cannot be downcast to `Self`
    fn apply(&mut self, value: &dyn PartialReflect) {
        PartialReflect::try_apply(self, value).unwrap();
    }

    /// Tries to [`apply`](PartialReflect::apply) a reflected value to this value.
    ///
    /// Functions the same as the [`apply`](PartialReflect::apply) function but returns an error instead of
    /// panicking.
    ///
    /// # Handling Errors
    ///
    /// This function may leave `self` in a partially mutated state if a error was encountered on the way.
    /// consider maintaining a cloned instance of this data you can switch to if a error is encountered.
    fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError>;

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

    /// Converts this reflected value into its dynamic representation based on its [kind].
    ///
    /// For example, a [`List`] type will internally invoke [`List::to_dynamic_list`], returning [`DynamicList`].
    /// A [`Struct`] type will invoke [`Struct::to_dynamic_struct`], returning [`DynamicStruct`].
    /// And so on.
    ///
    /// If the [kind] is [opaque], then the value will attempt to be cloned directly via [`reflect_clone`],
    /// since opaque types do not have any standard dynamic representation.
    ///
    /// To attempt to clone the value directly such that it returns a concrete instance of this type,
    /// use [`reflect_clone`].
    ///
    /// # Panics
    ///
    /// This method will panic if the [kind] is [opaque] and the call to [`reflect_clone`] fails.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::{PartialReflect};
    /// let value = (1, true, 3.14);
    /// let dynamic_value = value.to_dynamic();
    /// assert!(dynamic_value.is_dynamic())
    /// ```
    ///
    /// [kind]: PartialReflect::reflect_kind
    /// [`List`]: crate::List
    /// [`List::to_dynamic_list`]: crate::List::to_dynamic_list
    /// [`DynamicList`]: crate::DynamicList
    /// [`Struct`]: crate::Struct
    /// [`Struct::to_dynamic_struct`]: crate::Struct::to_dynamic_struct
    /// [`DynamicStruct`]: crate::DynamicStruct
    /// [opaque]: crate::ReflectKind::Opaque
    /// [`reflect_clone`]: PartialReflect::reflect_clone
    fn to_dynamic(&self) -> Box<dyn PartialReflect> {
        match self.reflect_ref() {
            ReflectRef::Struct(dyn_struct) => Box::new(dyn_struct.to_dynamic_struct()),
            ReflectRef::TupleStruct(dyn_tuple_struct) => {
                Box::new(dyn_tuple_struct.to_dynamic_tuple_struct())
            }
            ReflectRef::Tuple(dyn_tuple) => Box::new(dyn_tuple.to_dynamic_tuple()),
            ReflectRef::List(dyn_list) => Box::new(dyn_list.to_dynamic_list()),
            ReflectRef::Array(dyn_array) => Box::new(dyn_array.to_dynamic_array()),
            ReflectRef::Map(dyn_map) => Box::new(dyn_map.to_dynamic_map()),
            ReflectRef::Set(dyn_set) => Box::new(dyn_set.to_dynamic_set()),
            ReflectRef::Enum(dyn_enum) => Box::new(dyn_enum.to_dynamic_enum()),
            #[cfg(feature = "functions")]
            ReflectRef::Function(dyn_function) => Box::new(dyn_function.to_dynamic_function()),
            ReflectRef::Opaque(value) => value.reflect_clone().unwrap().into_partial_reflect(),
        }
    }

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
    /// # use bevy_reflect::PartialReflect;
    /// let value = (1, true, 3.14);
    /// let cloned = value.reflect_clone().unwrap();
    /// assert!(cloned.is::<(i32, bool, f64)>())
    /// ```
    ///
    /// [`to_dynamic`]: PartialReflect::to_dynamic
    fn reflect_clone(&self) -> Result<Box<dyn Reflect>, ReflectCloneError> {
        Err(ReflectCloneError::NotImplemented {
            type_path: Cow::Owned(self.reflect_type_path().to_string()),
        })
    }

    /// For a type implementing [`PartialReflect`], combines `reflect_clone` and
    /// `take` in a useful fashion, automatically constructing an appropriate
    /// [`ReflectCloneError`] if the downcast fails.
    ///
    /// This is an associated function, rather than a method, because methods
    /// with generic types prevent dyn-compatibility.
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
    fn reflect_partial_eq(&self, _value: &dyn PartialReflect) -> Option<bool> {
        None
    }

    /// Debug formatter for the value.
    ///
    /// Any value that is not an implementor of other `Reflect` subtraits
    /// (e.g. [`List`], [`Map`]), will default to the format: `"Reflect(type_path)"`,
    /// where `type_path` is the [type path] of the underlying type.
    ///
    /// [`List`]: crate::List
    /// [`Map`]: crate::Map
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
            #[cfg(feature = "functions")]
            ReflectRef::Function(dyn_function) => dyn_function.fmt(f),
            ReflectRef::Opaque(_) => write!(f, "Reflect({})", self.reflect_type_path()),
        }
    }

    /// Indicates whether or not this type is a _dynamic_ type.
    ///
    /// Dynamic types include the ones built-in to this [crate],
    /// such as [`DynamicStruct`], [`DynamicList`], and [`DynamicTuple`].
    /// However, they may be custom types used as proxies for other types
    /// or to facilitate scripting capabilities.
    ///
    /// By default, this method will return `false`.
    ///
    /// [`DynamicStruct`]: crate::DynamicStruct
    /// [`DynamicList`]: crate::DynamicList
    /// [`DynamicTuple`]: crate::DynamicTuple
    fn is_dynamic(&self) -> bool {
        false
    }
}

/// A core trait of [`bevy_reflect`], used for downcasting to concrete types.
///
/// This is a subtrait of [`PartialReflect`],
/// meaning any type which implements `Reflect` implements `PartialReflect` by definition.
///
/// It's recommended to use [the derive macro] rather than manually implementing this trait.
/// Doing so will automatically implement this trait, [`PartialReflect`], and many other useful traits for reflection,
/// including one of the appropriate subtraits: [`Struct`], [`TupleStruct`] or [`Enum`].
///
/// If you need to use this trait as a generic bound along with other reflection traits,
/// for your convenience, consider using [`Reflectable`] instead.
///
/// See the [crate-level documentation] to see how this trait can be used.
///
/// [`bevy_reflect`]: crate
/// [the derive macro]: bevy_reflect_derive::Reflect
/// [`Struct`]: crate::Struct
/// [`TupleStruct`]: crate::TupleStruct
/// [`Enum`]: crate::Enum
/// [`Reflectable`]: crate::Reflectable
/// [crate-level documentation]: crate
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not implement `Reflect` so cannot be fully reflected",
    note = "consider annotating `{Self}` with `#[derive(Reflect)]`"
)]
pub trait Reflect: PartialReflect + DynamicTyped + Any {
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

impl dyn PartialReflect {
    /// Returns `true` if the underlying value represents a value of type `T`, or `false`
    /// otherwise.
    ///
    /// Read `is` for more information on underlying values and represented types.
    #[inline]
    pub fn represents<T: Reflect + TypePath>(&self) -> bool {
        self.get_represented_type_info()
            .is_some_and(|t| t.type_path() == T::type_path())
    }

    /// Downcasts the value to type `T`, consuming the trait object.
    ///
    /// If the underlying value does not implement [`Reflect`]
    /// or is not of type `T`, returns `Err(self)`.
    ///
    /// For remote types, `T` should be the type itself rather than the wrapper type.
    pub fn try_downcast<T: Any>(
        self: Box<dyn PartialReflect>,
    ) -> Result<Box<T>, Box<dyn PartialReflect>> {
        self.try_into_reflect()?
            .downcast()
            .map_err(PartialReflect::into_partial_reflect)
    }

    /// Downcasts the value to type `T`, unboxing and consuming the trait object.
    ///
    /// If the underlying value does not implement [`Reflect`]
    /// or is not of type `T`, returns `Err(self)`.
    ///
    /// For remote types, `T` should be the type itself rather than the wrapper type.
    pub fn try_take<T: Any>(self: Box<dyn PartialReflect>) -> Result<T, Box<dyn PartialReflect>> {
        self.try_downcast().map(|value| *value)
    }

    /// Downcasts the value to type `T` by reference.
    ///
    /// If the underlying value does not implement [`Reflect`]
    /// or is not of type `T`, returns [`None`].
    ///
    /// For remote types, `T` should be the type itself rather than the wrapper type.
    pub fn try_downcast_ref<T: Any>(&self) -> Option<&T> {
        self.try_as_reflect()?.downcast_ref()
    }

    /// Downcasts the value to type `T` by mutable reference.
    ///
    /// If the underlying value does not implement [`Reflect`]
    /// or is not of type `T`, returns [`None`].
    ///
    /// For remote types, `T` should be the type itself rather than the wrapper type.
    pub fn try_downcast_mut<T: Any>(&mut self) -> Option<&mut T> {
        self.try_as_reflect_mut()?.downcast_mut()
    }
}

impl Debug for dyn PartialReflect {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.debug(f)
    }
}

// The following implementation never actually shadows the concrete TypePath implementation.
// See the comment on `dyn Reflect`'s `TypePath` implementation.
impl TypePath for dyn PartialReflect {
    fn type_path() -> &'static str {
        "dyn bevy_reflect::PartialReflect"
    }

    fn short_type_path() -> &'static str {
        "dyn PartialReflect"
    }
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
    /// it can be downcasted to. In the case that this underlying value "represents"
    /// a different type, like the Dynamic\*\*\* types do, you can call `represents`
    /// to determine what type they represent. Represented types cannot be downcasted
    /// to, but you can use [`FromReflect`] to create a value of the represented type from them.
    ///
    /// For remote types, `T` should be the type itself rather than the wrapper type.
    ///
    /// [`FromReflect`]: crate::FromReflect
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

macro_rules! impl_full_reflect {
    ($(<$($id:ident),* $(,)?>)? for $ty:ty $(where $($tt:tt)*)?) => {
        impl $(<$($id),*>)? $crate::Reflect for $ty $(where $($tt)*)? {
            fn into_any(self: bevy_platform::prelude::Box<Self>) -> bevy_platform::prelude::Box<dyn ::core::any::Any> {
                self
            }

            fn as_any(&self) -> &dyn ::core::any::Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn ::core::any::Any {
                self
            }

            fn into_reflect(self: bevy_platform::prelude::Box<Self>) -> bevy_platform::prelude::Box<dyn $crate::Reflect> {
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
        }
    };
}

pub(crate) use impl_full_reflect;

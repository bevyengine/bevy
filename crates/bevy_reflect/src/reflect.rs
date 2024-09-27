use crate::{
    array_debug, enum_debug, list_debug, map_debug, serde::Serializable, set_debug, struct_debug,
    tuple_debug, tuple_struct_debug, DynamicTypePath, DynamicTyped, OpaqueInfo, ReflectKind,
    ReflectKindMismatchError, ReflectMut, ReflectOwned, ReflectRef, TypeInfo, TypePath, Typed,
};
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
    /// Attempted to apply the wrong [kind](ReflectKind) to a type, e.g. a struct to a enum.
    MismatchedKinds {
        from_kind: ReflectKind,
        to_kind: ReflectKind,
    },

    #[error("enum variant `{variant_name}` doesn't have a field named `{field_name}`")]
    /// Enum variant that we tried to apply to was missing a field.
    MissingEnumField {
        variant_name: Box<str>,
        field_name: Box<str>,
    },

    #[error("`{from_type}` is not `{to_type}`")]
    /// Tried to apply incompatible types.
    MismatchedTypes {
        from_type: Box<str>,
        to_type: Box<str>,
    },

    #[error("attempted to apply type with {from_size} size to a type with {to_size} size")]
    /// Attempted to apply to types with mismatched sizez, e.g. a [u8; 4] to [u8; 3].
    DifferentSize { from_size: usize, to_size: usize },

    #[error("variant with name `{variant_name}` does not exist on enum `{enum_name}`")]
    /// The enum we tried to apply to didn't contain a variant with the give name.
    UnknownVariant {
        enum_name: Box<str>,
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
    /// If a type implements an [introspection subtrait], then the semantics of this
    /// method are as follows:
    /// - If `T` is a [`Struct`], then the value of each named field of `value` is
    ///   applied to the corresponding named field of `self`. Fields which are
    ///   not present in both structs are ignored.
    /// - If `T` is a [`TupleStruct`] or [`Tuple`], then the value of each
    ///   numbered field is applied to the corresponding numbered field of
    ///   `self.` Fields which are not present in both values are ignored.
    /// - If `T` is an [`Enum`], then the variant of `self` is `updated` to match
    ///   the variant of `value`. The corresponding fields of that variant are
    ///   applied from `value` onto `self`. Fields which are not present in both
    ///   values are ignored.
    /// - If `T` is a [`List`] or [`Array`], then each element of `value` is applied
    ///   to the corresponding element of `self`. Up to `self.len()` items are applied,
    ///   and excess elements in `value` are appended to `self`.
    /// - If `T` is a [`Map`], then for each key in `value`, the associated
    ///   value is applied to the value associated with the same key in `self`.
    ///   Keys which are not present in `self` are inserted.
    /// - If `T` is none of these, then `value` is downcast to `T`, cloned, and
    ///   assigned to `self`.
    ///
    /// Note that `Reflect` must be implemented manually for [`List`]s and
    /// [`Map`]s in order to achieve the correct semantics, as derived
    /// implementations will have the semantics for [`Struct`], [`TupleStruct`], [`Enum`]
    /// or none of the above depending on the kind of type. For lists and maps, use the
    /// [`list_apply`] and [`map_apply`] helper functions when implementing this method.
    ///
    /// [introspection subtrait]: crate#the-introspection-subtraits
    /// [`Struct`]: crate::Struct
    /// [`TupleStruct`]: crate::TupleStruct
    /// [`Tuple`]: crate::Tuple
    /// [`Enum`]: crate::Enum
    /// [`List`]: crate::List
    /// [`Array`]: crate::Array
    /// [`Map`]: crate::Map
    /// [`list_apply`]: crate::list_apply
    /// [`map_apply`]: crate::map_apply
    ///
    /// # Panics
    ///
    /// Derived implementations of this method will panic:
    /// - If the type of `value` is not of the same kind as `T` (e.g. if `T` is
    ///   a `List`, while `value` is a `Struct`).
    /// - If `T` is any complex type and the corresponding fields or elements of
    ///   `self` and `value` are not of the same type.
    /// - If `T` is an opaque type and `self` cannot be downcast to `T`
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
    fn reflect_ref(&self) -> ReflectRef;

    /// Returns a mutable enumeration of "kinds" of type.
    ///
    /// See [`ReflectMut`].
    fn reflect_mut(&mut self) -> ReflectMut;

    /// Returns an owned enumeration of "kinds" of type.
    ///
    /// See [`ReflectOwned`].
    fn reflect_owned(self: Box<Self>) -> ReflectOwned;

    /// Clones the value as a `Reflect` trait object.
    ///
    /// When deriving `Reflect` for a struct, tuple struct or enum, the value is
    /// cloned via [`Struct::clone_dynamic`], [`TupleStruct::clone_dynamic`],
    /// or [`Enum::clone_dynamic`], respectively.
    /// Implementors of other `Reflect` subtraits (e.g. [`List`], [`Map`]) should
    /// use those subtraits' respective `clone_dynamic` methods.
    ///
    /// [`Struct::clone_dynamic`]: crate::Struct::clone_dynamic
    /// [`TupleStruct::clone_dynamic`]: crate::TupleStruct::clone_dynamic
    /// [`Enum::clone_dynamic`]: crate::Enum::clone_dynamic
    /// [`List`]: crate::List
    /// [`Map`]: crate::Map
    fn clone_value(&self) -> Box<dyn PartialReflect>;

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

    /// Returns a serializable version of the value.
    ///
    /// If the underlying type does not support serialization, returns `None`.
    fn serializable(&self) -> Option<Serializable> {
        None
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
    /// Returns the value as a [`Box<dyn Any>`][std::any::Any].
    ///
    /// For remote wrapper types, this will return the remote type instead.
    fn into_any(self: Box<Self>) -> Box<dyn Any>;

    /// Returns the value as a [`&dyn Any`][std::any::Any].
    ///
    /// For remote wrapper types, this will return the remote type instead.
    fn as_any(&self) -> &dyn Any;

    /// Returns the value as a [`&mut dyn Any`][std::any::Any].
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
            .map(|t| t.type_path() == T::type_path())
            .unwrap_or(false)
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
            fn into_any(self: Box<Self>) -> Box<dyn ::core::any::Any> {
                self
            }

            fn as_any(&self) -> &dyn ::core::any::Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn ::core::any::Any {
                self
            }

            fn into_reflect(self: Box<Self>) -> Box<dyn $crate::Reflect> {
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
                value: Box<dyn $crate::Reflect>,
            ) -> Result<(), Box<dyn $crate::Reflect>> {
                *self = <dyn $crate::Reflect>::take(value)?;
                Ok(())
            }
        }
    };
}

pub(crate) use impl_full_reflect;

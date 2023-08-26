use crate::{
    array_debug, enum_debug, list_debug, map_debug, serde::Serializable, struct_debug, tuple_debug,
    tuple_struct_debug, Array, DynamicTypePath, Enum, List, Map, Struct, Tuple, TupleStruct,
    TypeInfo, Typed, ValueInfo,
};
use std::{
    any::{self, Any, TypeId},
    fmt::Debug,
};

use crate::utility::NonGenericTypeInfoCell;

/// An immutable enumeration of "kinds" of reflected type.
///
/// Each variant contains a trait object with methods specific to a kind of
/// type.
///
/// A `ReflectRef` is obtained via [`Reflect::reflect_ref`].
pub enum ReflectRef<'a> {
    Struct(&'a dyn Struct),
    TupleStruct(&'a dyn TupleStruct),
    Tuple(&'a dyn Tuple),
    List(&'a dyn List),
    Array(&'a dyn Array),
    Map(&'a dyn Map),
    Enum(&'a dyn Enum),
    Value(&'a dyn Reflect),
}

/// A mutable enumeration of "kinds" of reflected type.
///
/// Each variant contains a trait object with methods specific to a kind of
/// type.
///
/// A `ReflectMut` is obtained via [`Reflect::reflect_mut`].
pub enum ReflectMut<'a> {
    Struct(&'a mut dyn Struct),
    TupleStruct(&'a mut dyn TupleStruct),
    Tuple(&'a mut dyn Tuple),
    List(&'a mut dyn List),
    Array(&'a mut dyn Array),
    Map(&'a mut dyn Map),
    Enum(&'a mut dyn Enum),
    Value(&'a mut dyn Reflect),
}

/// An owned enumeration of "kinds" of reflected type.
///
/// Each variant contains a trait object with methods specific to a kind of
/// type.
///
/// A `ReflectOwned` is obtained via [`Reflect::reflect_owned`].
pub enum ReflectOwned {
    Struct(Box<dyn Struct>),
    TupleStruct(Box<dyn TupleStruct>),
    Tuple(Box<dyn Tuple>),
    List(Box<dyn List>),
    Array(Box<dyn Array>),
    Map(Box<dyn Map>),
    Enum(Box<dyn Enum>),
    Value(Box<dyn Reflect>),
}

/// The core trait of [`bevy_reflect`], used for accessing and modifying data dynamically.
///
/// It's recommended to use the [derive macro] rather than manually implementing this trait.
/// Doing so will automatically implement many other useful traits for reflection,
/// including one of the appropriate subtraits: [`Struct`], [`TupleStruct`] or [`Enum`].
///
/// See the [crate-level documentation] to see how this trait and its subtraits can be used.
///
/// [`bevy_reflect`]: crate
/// [derive macro]: bevy_reflect_derive::Reflect
/// [crate-level documentation]: crate
pub trait Reflect: DynamicTypePath + Any + Send + Sync {
    /// Returns the [type name][std::any::type_name] of the underlying type.
    fn type_name(&self) -> &str;

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

    /// Returns the value as a [`Box<dyn Any>`][std::any::Any].
    fn into_any(self: Box<Self>) -> Box<dyn Any>;

    /// Returns the value as a [`&dyn Any`][std::any::Any].
    fn as_any(&self) -> &dyn Any;

    /// Returns the value as a [`&mut dyn Any`][std::any::Any].
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Casts this type to a boxed reflected value.
    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect>;

    /// Casts this type to a reflected value.
    fn as_reflect(&self) -> &dyn Reflect;

    /// Casts this type to a mutable reflected value.
    fn as_reflect_mut(&mut self) -> &mut dyn Reflect;

    /// Applies a reflected value to this value.
    ///
    /// If a type implements a subtrait of `Reflect`, then the semantics of this
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
    /// - If `T` is a value type and `self` cannot be downcast to `T`
    fn apply(&mut self, value: &dyn Reflect);

    /// Performs a type-checked assignment of a reflected value to this value.
    ///
    /// If `value` does not contain a value of type `T`, returns an `Err`
    /// containing the trait object.
    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>>;

    /// Returns an enumeration of "kinds" of type.
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
    fn clone_value(&self) -> Box<dyn Reflect>;

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

    /// Debug formatter for the value.
    ///
    /// Any value that is not an implementor of other `Reflect` subtraits
    /// (e.g. [`List`], [`Map`]), will default to the format: `"Reflect(type_name)"`,
    /// where `type_name` is the [type name] of the underlying type.
    ///
    /// [type name]: Self::type_name
    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.reflect_ref() {
            ReflectRef::Struct(dyn_struct) => struct_debug(dyn_struct, f),
            ReflectRef::TupleStruct(dyn_tuple_struct) => tuple_struct_debug(dyn_tuple_struct, f),
            ReflectRef::Tuple(dyn_tuple) => tuple_debug(dyn_tuple, f),
            ReflectRef::List(dyn_list) => list_debug(dyn_list, f),
            ReflectRef::Array(dyn_array) => array_debug(dyn_array, f),
            ReflectRef::Map(dyn_map) => map_debug(dyn_map, f),
            ReflectRef::Enum(dyn_enum) => enum_debug(dyn_enum, f),
            _ => write!(f, "Reflect({})", self.type_name()),
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

impl Debug for dyn Reflect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.debug(f)
    }
}

impl Typed for dyn Reflect {
    fn type_info() -> &'static TypeInfo {
        static CELL: NonGenericTypeInfoCell = NonGenericTypeInfoCell::new();
        CELL.get_or_set(|| TypeInfo::Value(ValueInfo::new::<Self>()))
    }
}

#[deny(rustdoc::broken_intra_doc_links)]
impl dyn Reflect {
    /// Downcasts the value to type `T`, consuming the trait object.
    ///
    /// If the underlying value is not of type `T`, returns `Err(self)`.
    pub fn downcast<T: Reflect>(self: Box<dyn Reflect>) -> Result<Box<T>, Box<dyn Reflect>> {
        if self.is::<T>() {
            Ok(self.into_any().downcast().unwrap())
        } else {
            Err(self)
        }
    }

    /// Downcasts the value to type `T`, unboxing and consuming the trait object.
    ///
    /// If the underlying value is not of type `T`, returns `Err(self)`.
    pub fn take<T: Reflect>(self: Box<dyn Reflect>) -> Result<T, Box<dyn Reflect>> {
        self.downcast::<T>().map(|value| *value)
    }

    /// Returns `true` if the underlying value represents a value of type `T`, or `false`
    /// otherwise.
    ///
    /// Read `is` for more information on underlying values and represented types.
    #[inline]
    pub fn represents<T: Reflect>(&self) -> bool {
        self.type_name() == any::type_name::<T>()
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
    /// [`FromReflect`]: crate::FromReflect
    #[inline]
    pub fn is<T: Reflect>(&self) -> bool {
        self.type_id() == TypeId::of::<T>()
    }

    /// Downcasts the value to type `T` by reference.
    ///
    /// If the underlying value is not of type `T`, returns `None`.
    #[inline]
    pub fn downcast_ref<T: Reflect>(&self) -> Option<&T> {
        self.as_any().downcast_ref::<T>()
    }

    /// Downcasts the value to type `T` by mutable reference.
    ///
    /// If the underlying value is not of type `T`, returns `None`.
    #[inline]
    pub fn downcast_mut<T: Reflect>(&mut self) -> Option<&mut T> {
        self.as_any_mut().downcast_mut::<T>()
    }
}

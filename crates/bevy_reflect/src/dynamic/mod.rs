//! Contains types for holding dynamic representations of other types.

use core::{
    any::Any,
    cmp::Ordering,
    fmt::{Debug, Formatter},
    mem::transmute,
    ops::Deref,
};

use crate::{
    array::{
        array_hash, array_like_partial_cmp, array_like_partial_eq, array_partial_cmp,
        array_partial_eq, array_try_apply, dynamic_array_hash, Array, ArrayInfo, ArrayIter,
    },
    enums::{
        dynamic_enum_hash, enum_hash, enum_like_partial_cmp, enum_like_partial_eq,
        enum_partial_cmp, enum_partial_eq, Enum, VariantFieldIter,
    },
    list::{
        dynamic_list_hash, list_hash, list_like_partial_cmp, list_like_partial_eq,
        list_partial_cmp, list_partial_eq, list_try_apply, List, ListInfo, ListIter,
    },
    map::{
        map_like_partial_cmp, map_like_partial_eq, map_partial_cmp, map_partial_eq, map_try_apply,
        Map, MapInfo,
    },
    set::{set_like_partial_eq, set_partial_eq, set_try_apply, Set, SetInfo},
    structs::{
        struct_like_partial_cmp, struct_like_partial_eq, struct_partial_cmp, struct_partial_eq,
        FieldIter, Struct, StructInfo,
    },
    tuple::{
        tuple_like_partial_cmp, tuple_like_partial_eq, tuple_partial_cmp, tuple_partial_eq,
        tuple_try_apply, Tuple, TupleFieldIter, TupleInfo,
    },
    tuple_struct::{
        tuple_struct_like_partial_cmp, tuple_struct_like_partial_eq, tuple_struct_partial_cmp,
        tuple_struct_partial_eq, TupleStruct, TupleStructFieldIter, TupleStructInfo,
    },
    ApplyError, PartialReflect, Reflect, ReflectCloneError, ReflectKind, ReflectMut, ReflectOwned,
    ReflectRef, TypeInfo, TypePath,
};

use alloc::{
    borrow::Cow,
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use bevy_platform::collections::{
    hash_table::OccupiedEntry as HashTableOccupiedEntry, HashMap, HashTable,
};
use bevy_reflect_derive::impl_type_path;

mod enum_;

pub use enum_::*;

/// A dynamically typed value based on reflection.
///
/// A value can either be a "concrete" [`Reflect`] value, or it may be a dynamic representation such
/// as [`DynamicStruct`].
pub enum DynamicValue {
    /// A dynamically-typed struct value.
    Struct(DynamicStruct),
    /// A dynamically-typed tuple-struct value.
    TupleStruct(DynamicTupleStruct),
    /// A dynamically-typed tuple value.
    Tuple(DynamicTuple),
    /// A dynamically-typed list value.
    List(DynamicList),
    /// A dynamically-typed array value.
    Array(DynamicArray),
    /// A dynamically-typed map value.
    Map(DynamicMap),
    /// A dynamically-typed set value.
    Set(DynamicSet),
    /// A dynamically-typed enum value.
    Enum(DynamicEnum),
    /// A concrete reflected value.
    Concrete(Box<dyn Reflect>),
}

impl DynamicValue {
    /// Creates a dynamically-typed value equivalent to the provided [`PartialReflect`].
    ///
    /// This function clones opaque fields out of the original `value`, which can fail if the
    /// original type does not implement [`Clone`].
    pub fn try_from_reflect(value: &dyn PartialReflect) -> Result<Self, ReflectCloneError> {
        Ok(match value.reflect_ref() {
            ReflectRef::Struct(value) => Self::Struct(DynamicStruct::try_from_reflect(value)?),
            ReflectRef::TupleStruct(value) => {
                Self::TupleStruct(DynamicTupleStruct::try_from_reflect(value)?)
            }
            ReflectRef::Tuple(value) => Self::Tuple(DynamicTuple::try_from_reflect(value)?),
            ReflectRef::List(value) => Self::List(DynamicList::try_from_reflect(value)?),
            ReflectRef::Array(value) => Self::Array(DynamicArray::try_from_reflect(value)?),
            ReflectRef::Map(value) => Self::Map(DynamicMap::try_from_reflect(value)?),
            ReflectRef::Set(value) => Self::Set(DynamicSet::try_from_reflect(value)?),
            ReflectRef::Enum(value) => Self::Enum(DynamicEnum::try_from_ref(value)?),
            #[cfg(feature = "functions")]
            ReflectRef::Function(value) => todo!(),
            ReflectRef::Opaque(value) => Self::Concrete(value.reflect_clone()?),
        })
    }

    /// Apply the values in `self` to the provided `reflect` using reflection.
    ///
    /// If this function fails, the `reflect` may be partially mutated.
    pub fn try_apply(self, reflect: &mut dyn Reflect) -> Result<(), ApplyError> {
        match self {
            Self::Struct(value) => value.try_apply(reflect.reflect_mut().as_struct()?),
            Self::TupleStruct(value) => value.try_apply(reflect.reflect_mut().as_tuple_struct()?),
            Self::Tuple(value) => value.try_apply(reflect.reflect_mut().as_tuple()?),
            Self::List(value) => value.try_apply(reflect.reflect_mut().as_list()?),
            Self::Array(value) => value.try_apply(reflect.reflect_mut().as_array()?),
            Self::Map(value) => value.try_apply(reflect.reflect_mut().as_map()?),
            Self::Set(value) => value.try_apply(reflect.reflect_mut().as_set()?),
            Self::Enum(value) => value.try_apply(reflect.reflect_mut().as_enum()?),
            Self::Concrete(value) => {
                let value_type = value.reflect_type_info().type_path();
                reflect.set(value).map_err(|_| ApplyError::MismatchedTypes {
                    from_type: value_type.into(),
                    to_type: reflect.reflect_type_path().into(),
                })
            }
        }
    }

    /// Attempts to take a concrete value of type `T` out of this possibly-dynamic value.
    ///
    /// Returns [`Self`] if this is a dynamic representation of a value, or if the stored concrete
    /// value's type does not } match `T`.
    pub fn try_take<T: Any>(self) -> Result<T, Self> {
        match self {
            Self::Concrete(value) => match value.downcast() {
                Ok(value) => Ok(*value),
                Err(value) => Err(Self::Concrete(value)),
            },
            _ => Err(self),
        }
    }

    /// Converts this value into an equivalent [`PartialReflect`] implementation.
    ///
    /// [`Self::Concrete`] values are passed directly, while for all dynamic representations, this
    /// value is boxed and returned as implementing [`PartialReflect`].
    pub fn as_boxed_partial_reflect(self) -> Box<dyn PartialReflect> {
        match self {
            Self::Concrete(inner_box) => inner_box,
            _ => DynamicValueWrapper::from_box(Box::new(self)),
        }
    }

    /// Converts this value into an equivalent [`PartialReflect`] implementation.
    ///
    /// [`Self::Concrete`] values are passed directly, while for all dynamic representations, this
    /// value is wrapped and returned as implementing [`PartialReflect`].
    pub fn as_partial_reflect(&self) -> &dyn PartialReflect {
        match self {
            Self::Concrete(inner_box) => &**inner_box,
            _ => DynamicValueWrapper::from_ref(self),
        }
    }

    /// Converts this value into an equivalent [`PartialReflect`] implementation.
    ///
    /// [`Self::Concrete`] values are passed directly, while for all dynamic representations, this
    /// value is wrapped and returned as implementing [`PartialReflect`].
    pub fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
        match self {
            Self::Concrete(inner_box) => &mut **inner_box,
            _ => DynamicValueWrapper::from_mut(self),
        }
    }

    /// Attempts to clone this value's data.
    ///
    /// Note: Any [`Self::Concrete`] variants first call [`PartialReflect::reflect_clone`], and then
    /// fallback to converting the value to a dynamic representation.
    pub fn dynamic_clone(&self) -> Result<Self, ReflectCloneError> {
        self.as_ref().dynamic_clone()
    }

    /// Gets the type that this dynamic representation represents.
    pub fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        self.as_ref().get_represented_type_into()
    }

    /// Returns `true` if the underlying value represents a value of type `T`, or `false` otherwise.
    pub fn represents<T: Reflect + TypePath>(&self) -> bool {
        self.as_ref().represents::<T>()
    }

    /// Gets the concrete value if this value is [`Self::Concrete`], returns [`None`] otherwise.
    pub fn as_concrete(&self) -> Option<&dyn Reflect> {
        match self {
            Self::Concrete(value) => Some(&**value),
            _ => None,
        }
    }

    /// Gets the concrete value if this value is [`Self::Concrete`], returns [`None`] otherwise.
    pub fn as_concrete_mut(&mut self) -> Option<&mut dyn Reflect> {
        match self {
            Self::Concrete(value) => Some(&mut **value),
            _ => None,
        }
    }

    /// Converts this dynamic value into its reference form.
    pub fn as_ref(&self) -> DynamicValueRef<'_> {
        DynamicValueRef::from(self)
    }

    /// Converts this dynamic value into its mutable reference form.
    pub fn as_mut(&mut self) -> DynamicValueMut<'_> {
        DynamicValueMut::from(self)
    }
}

impl Debug for DynamicValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            DynamicValue::Struct(value) => value.fmt(f),
            DynamicValue::TupleStruct(value) => value.fmt(f),
            DynamicValue::Tuple(value) => value.fmt(f),
            DynamicValue::List(value) => value.fmt(f),
            DynamicValue::Array(value) => value.fmt(f),
            DynamicValue::Map(value) => value.fmt(f),
            DynamicValue::Set(value) => value.fmt(f),
            DynamicValue::Enum(value) => value.fmt(f),
            DynamicValue::Concrete(value) => value.debug(f),
        }
    }
}

impl From<DynamicStruct> for DynamicValue {
    fn from(value: DynamicStruct) -> Self {
        Self::Struct(value)
    }
}

impl From<DynamicTupleStruct> for DynamicValue {
    fn from(value: DynamicTupleStruct) -> Self {
        Self::TupleStruct(value)
    }
}

impl From<DynamicTuple> for DynamicValue {
    fn from(value: DynamicTuple) -> Self {
        Self::Tuple(value)
    }
}

impl From<DynamicList> for DynamicValue {
    fn from(value: DynamicList) -> Self {
        Self::List(value)
    }
}

impl From<DynamicArray> for DynamicValue {
    fn from(value: DynamicArray) -> Self {
        Self::Array(value)
    }
}

impl From<DynamicMap> for DynamicValue {
    fn from(value: DynamicMap) -> Self {
        Self::Map(value)
    }
}

impl From<DynamicSet> for DynamicValue {
    fn from(value: DynamicSet) -> Self {
        Self::Set(value)
    }
}

impl From<DynamicEnum> for DynamicValue {
    fn from(value: DynamicEnum) -> Self {
        Self::Enum(value)
    }
}

impl<T: Reflect> From<T> for DynamicValue {
    fn from(value: T) -> Self {
        Self::Concrete(Box::new(value))
    }
}

impl From<Box<dyn Reflect>> for DynamicValue {
    fn from(value: Box<dyn Reflect>) -> Self {
        Self::Concrete(value)
    }
}

/// A dynamically typed reference based on reflection.
///
/// A value can either be a "concrete" [`Reflect`] value, or it may be a dynamic representation such
/// as [`DynamicStruct`].
///
/// This may be a reference into a [`DynamicValue`], but also allows any [`Reflect`] value to be
/// referenced.
#[derive(Clone, Copy)]
pub enum DynamicValueRef<'a> {
    /// A dynamically-typed struct value.
    Struct(&'a DynamicStruct),
    /// A dynamically-typed tuple-struct value.
    TupleStruct(&'a DynamicTupleStruct),
    /// A dynamically-typed tuple value.
    Tuple(&'a DynamicTuple),
    /// A dynamically-typed list value.
    List(&'a DynamicList),
    /// A dynamically-typed array value.
    Array(&'a DynamicArray),
    /// A dynamically-typed map value.
    Map(&'a DynamicMap),
    /// A dynamically-typed set value.
    Set(&'a DynamicSet),
    /// A dynamically-typed enum value.
    Enum(&'a DynamicEnum),
    /// A concrete reflected value.
    Concrete(&'a dyn Reflect),
}

impl<'b> DynamicValueRef<'b> {
    /// Attempts to clone this value's data.
    ///
    /// Note: Any [`Self::Concrete`] variants first call [`PartialReflect::reflect_clone`], and then
    /// fallback to converting the value to a dynamic representation.
    pub fn dynamic_clone(&self) -> Result<DynamicValue, ReflectCloneError> {
        match self {
            Self::Concrete(inner_box) => {
                if let Ok(value) = inner_box.reflect_clone() {
                    return Ok(DynamicValue::Concrete(value));
                }

                DynamicValue::try_from_reflect(&**inner_box)
            }
            Self::Struct(value) => Ok(DynamicValue::Struct(DynamicStruct::dynamic_clone(value)?)),
            Self::TupleStruct(value) => Ok(DynamicValue::TupleStruct(
                DynamicTupleStruct::dynamic_clone(value)?,
            )),
            Self::Tuple(value) => Ok(DynamicValue::Tuple(DynamicTuple::dynamic_clone(value)?)),
            Self::List(value) => Ok(DynamicValue::List(DynamicList::dynamic_clone(value)?)),
            Self::Array(value) => Ok(DynamicValue::Array(DynamicArray::dynamic_clone(value)?)),
            Self::Map(value) => Ok(DynamicValue::Map(DynamicMap::dynamic_clone(value)?)),
            Self::Set(value) => Ok(DynamicValue::Set(DynamicSet::dynamic_clone(value)?)),
            Self::Enum(value) => Ok(DynamicValue::Enum(DynamicEnum::dynamic_clone(value)?)),
        }
    }

    /// Computes the hash of this dynamic value.
    pub fn dynamic_hash(&self) -> Option<u64> {
        match self {
            Self::Struct(_)
            | Self::TupleStruct(_)
            | Self::Tuple(_)
            | Self::Map(_)
            | Self::Set(_) => None,
            Self::List(value) => dynamic_list_hash(value),
            Self::Array(value) => dynamic_array_hash(value),
            Self::Enum(value) => dynamic_enum_hash(value),
            Self::Concrete(value) => value.reflect_hash(),
        }
    }

    /// Compares whether two dynamic values are equivalent.
    ///
    /// This is the dynamic version of [`PartialEq::eq`].
    pub fn dynamic_partial_eq(&self, other: DynamicValueRef<'_>) -> Option<bool> {
        // Note: we can't use `Self` here since `Self` also carries along the lifetime. `*self` and
        // `other` both have different lifetimes though, so referencing `Self` in the match results
        // in the lifetimes being invalid. Naming `DynamicValueRef` directly allows the lifetime to
        // change as needed.
        match (*self, other) {
            (DynamicValueRef::Struct(left), DynamicValueRef::Struct(right)) => {
                struct_like_partial_eq(left, right)
            }
            (DynamicValueRef::TupleStruct(left), DynamicValueRef::TupleStruct(right)) => {
                tuple_struct_like_partial_eq(left, right)
            }
            (DynamicValueRef::Tuple(left), DynamicValueRef::Tuple(right)) => {
                tuple_like_partial_eq(left, right)
            }
            (DynamicValueRef::List(left), DynamicValueRef::List(right)) => {
                list_like_partial_eq(left, right)
            }
            (DynamicValueRef::Array(left), DynamicValueRef::Array(right)) => {
                array_like_partial_eq(left, right)
            }
            (DynamicValueRef::Map(left), DynamicValueRef::Map(right)) => {
                map_like_partial_eq(left, right)
            }
            (DynamicValueRef::Set(left), DynamicValueRef::Set(right)) => {
                set_like_partial_eq(left, right)
            }
            (DynamicValueRef::Enum(left), DynamicValueRef::Enum(right)) => {
                enum_like_partial_eq(left, right)
            }
            (DynamicValueRef::Concrete(left), DynamicValueRef::Concrete(right)) => {
                left.reflect_partial_eq(right)
            }
            (DynamicValueRef::Concrete(concrete), DynamicValueRef::Struct(dynamic))
            | (DynamicValueRef::Struct(dynamic), DynamicValueRef::Concrete(concrete)) => {
                let ReflectRef::Struct(concrete) = concrete.reflect_ref() else {
                    return None;
                };
                struct_like_partial_eq(concrete, dynamic)
            }
            (DynamicValueRef::Concrete(concrete), DynamicValueRef::TupleStruct(dynamic))
            | (DynamicValueRef::TupleStruct(dynamic), DynamicValueRef::Concrete(concrete)) => {
                let ReflectRef::TupleStruct(concrete) = concrete.reflect_ref() else {
                    return None;
                };
                tuple_struct_like_partial_eq(concrete, dynamic)
            }
            (DynamicValueRef::Concrete(concrete), DynamicValueRef::Tuple(dynamic))
            | (DynamicValueRef::Tuple(dynamic), DynamicValueRef::Concrete(concrete)) => {
                let ReflectRef::Tuple(concrete) = concrete.reflect_ref() else {
                    return None;
                };
                tuple_like_partial_eq(concrete, dynamic)
            }
            (DynamicValueRef::Concrete(concrete), DynamicValueRef::List(dynamic))
            | (DynamicValueRef::List(dynamic), DynamicValueRef::Concrete(concrete)) => {
                let ReflectRef::List(concrete) = concrete.reflect_ref() else {
                    return None;
                };
                list_like_partial_eq(concrete, dynamic)
            }
            (DynamicValueRef::Concrete(concrete), DynamicValueRef::Array(dynamic))
            | (DynamicValueRef::Array(dynamic), DynamicValueRef::Concrete(concrete)) => {
                let ReflectRef::Array(concrete) = concrete.reflect_ref() else {
                    return None;
                };
                array_like_partial_eq(concrete, dynamic)
            }
            (DynamicValueRef::Concrete(concrete), DynamicValueRef::Map(dynamic))
            | (DynamicValueRef::Map(dynamic), DynamicValueRef::Concrete(concrete)) => {
                let ReflectRef::Map(concrete) = concrete.reflect_ref() else {
                    return None;
                };
                map_like_partial_eq(concrete, dynamic)
            }
            (DynamicValueRef::Concrete(concrete), DynamicValueRef::Set(dynamic))
            | (DynamicValueRef::Set(dynamic), DynamicValueRef::Concrete(concrete)) => {
                let ReflectRef::Set(concrete) = concrete.reflect_ref() else {
                    return None;
                };
                set_like_partial_eq(concrete, dynamic)
            }
            (DynamicValueRef::Concrete(concrete), DynamicValueRef::Enum(dynamic))
            | (DynamicValueRef::Enum(dynamic), DynamicValueRef::Concrete(concrete)) => {
                let ReflectRef::Enum(concrete) = concrete.reflect_ref() else {
                    return None;
                };
                enum_like_partial_eq(concrete, dynamic)
            }
            _ => None,
        }
    }

    /// Compares two dynamic values to determine their order.
    ///
    /// This is the dynamic version of [`PartialOrd::partial_cmp`].
    pub fn dynamic_partial_cmp(&self, other: DynamicValueRef<'_>) -> Option<Ordering> {
        match (*self, other) {
            (Self::Struct(left), DynamicValueRef::Struct(right)) => {
                struct_like_partial_cmp(left, right)
            }
            (Self::TupleStruct(left), DynamicValueRef::TupleStruct(right)) => {
                tuple_struct_like_partial_cmp(left, right)
            }
            (Self::Tuple(left), DynamicValueRef::Tuple(right)) => {
                tuple_like_partial_cmp(left, right)
            }
            (Self::List(left), DynamicValueRef::List(right)) => list_like_partial_cmp(left, right),
            (Self::Array(left), DynamicValueRef::Array(right)) => {
                array_like_partial_cmp(left, right)
            }
            (Self::Map(left), DynamicValueRef::Map(right)) => map_like_partial_cmp(left, right),
            (Self::Enum(left), DynamicValueRef::Enum(right)) => enum_like_partial_cmp(left, right),
            (Self::Concrete(left), DynamicValueRef::Concrete(right)) => {
                left.reflect_partial_cmp(right)
            }
            // Sets don't impl partial_cmp.
            (Self::Set(_), _) | (_, DynamicValueRef::Set(_)) => None,
            (Self::Concrete(concrete), DynamicValueRef::Struct(dynamic)) => {
                let ReflectRef::Struct(concrete) = concrete.reflect_ref() else {
                    return None;
                };
                struct_like_partial_cmp(concrete, dynamic)
            }
            (Self::Concrete(concrete), DynamicValueRef::TupleStruct(dynamic)) => {
                let ReflectRef::TupleStruct(concrete) = concrete.reflect_ref() else {
                    return None;
                };
                tuple_struct_like_partial_cmp(concrete, dynamic)
            }
            (Self::Concrete(concrete), DynamicValueRef::Tuple(dynamic)) => {
                let ReflectRef::Tuple(concrete) = concrete.reflect_ref() else {
                    return None;
                };
                tuple_like_partial_cmp(concrete, dynamic)
            }
            (Self::Concrete(concrete), DynamicValueRef::List(dynamic)) => {
                let ReflectRef::List(concrete) = concrete.reflect_ref() else {
                    return None;
                };
                list_like_partial_cmp(concrete, dynamic)
            }
            (Self::Concrete(concrete), DynamicValueRef::Array(dynamic)) => {
                let ReflectRef::Array(concrete) = concrete.reflect_ref() else {
                    return None;
                };
                array_like_partial_cmp(concrete, dynamic)
            }
            (Self::Concrete(concrete), DynamicValueRef::Map(dynamic)) => {
                let ReflectRef::Map(concrete) = concrete.reflect_ref() else {
                    return None;
                };
                map_like_partial_cmp(concrete, dynamic)
            }
            (Self::Concrete(concrete), DynamicValueRef::Enum(dynamic)) => {
                let ReflectRef::Enum(concrete) = concrete.reflect_ref() else {
                    return None;
                };
                enum_like_partial_cmp(concrete, dynamic)
            }
            (Self::Struct(dynamic), DynamicValueRef::Concrete(concrete)) => {
                let ReflectRef::Struct(concrete) = concrete.reflect_ref() else {
                    return None;
                };
                struct_like_partial_cmp(dynamic, concrete)
            }
            (Self::TupleStruct(dynamic), DynamicValueRef::Concrete(concrete)) => {
                let ReflectRef::TupleStruct(concrete) = concrete.reflect_ref() else {
                    return None;
                };
                tuple_struct_like_partial_cmp(dynamic, concrete)
            }
            (Self::Tuple(dynamic), DynamicValueRef::Concrete(concrete)) => {
                let ReflectRef::Tuple(concrete) = concrete.reflect_ref() else {
                    return None;
                };
                tuple_like_partial_cmp(dynamic, concrete)
            }
            (Self::List(dynamic), DynamicValueRef::Concrete(concrete)) => {
                let ReflectRef::List(concrete) = concrete.reflect_ref() else {
                    return None;
                };
                list_like_partial_cmp(dynamic, concrete)
            }
            (Self::Array(dynamic), DynamicValueRef::Concrete(concrete)) => {
                let ReflectRef::Array(concrete) = concrete.reflect_ref() else {
                    return None;
                };
                array_like_partial_cmp(dynamic, concrete)
            }
            (Self::Map(dynamic), DynamicValueRef::Concrete(concrete)) => {
                let ReflectRef::Map(concrete) = concrete.reflect_ref() else {
                    return None;
                };
                map_like_partial_cmp(dynamic, concrete)
            }
            (Self::Enum(dynamic), DynamicValueRef::Concrete(concrete)) => {
                let ReflectRef::Enum(concrete) = concrete.reflect_ref() else {
                    return None;
                };
                enum_like_partial_cmp(dynamic, concrete)
            }
            _ => None,
        }
    }

    /// Gets the type that this dynamic representation represents.
    pub fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        match self {
            Self::Struct(value) => value.represented_type,
            Self::TupleStruct(value) => value.represented_type,
            Self::Tuple(value) => value.represented_type,
            Self::List(value) => value.represented_type,
            Self::Array(value) => value.represented_type,
            Self::Map(value) => value.represented_type,
            Self::Set(value) => value.represented_type,
            Self::Enum(value) => value.represented_type,
            Self::Concrete(value) => Some(value.reflect_type_info()),
        }
    }

    /// Returns `true` if the underlying value represents a value of type `T`, or `false` otherwise.
    pub fn represents<T: Reflect + TypePath>(&self) -> bool {
        self.get_represented_type_into()
            .is_some_and(|t| t.type_path() == T::type_path())
    }

    /// Gets the concrete value if this value is [`Self::Concrete`], returns [`None`] otherwise.
    pub fn as_concrete(&self) -> Option<&dyn Reflect> {
        match self {
            Self::Concrete(value) => Some(&**value),
            _ => None,
        }
    }
}

impl<'a> From<&'a DynamicValue> for DynamicValueRef<'a> {
    fn from(value: &'a DynamicValue) -> Self {
        match value {
            DynamicValue::Struct(value) => Self::Struct(value),
            DynamicValue::TupleStruct(value) => Self::TupleStruct(value),
            DynamicValue::Tuple(value) => Self::Tuple(value),
            DynamicValue::List(value) => Self::List(value),
            DynamicValue::Array(value) => Self::Array(value),
            DynamicValue::Map(value) => Self::Map(value),
            DynamicValue::Set(value) => Self::Set(value),
            DynamicValue::Enum(value) => Self::Enum(value),
            DynamicValue::Concrete(value) => Self::Concrete(&**value),
        }
    }
}

impl<'a> From<&'a DynamicStruct> for DynamicValueRef<'a> {
    fn from(value: &'a DynamicStruct) -> Self {
        Self::Struct(value)
    }
}

impl<'a> From<&'a DynamicTupleStruct> for DynamicValueRef<'a> {
    fn from(value: &'a DynamicTupleStruct) -> Self {
        Self::TupleStruct(value)
    }
}

impl<'a> From<&'a DynamicTuple> for DynamicValueRef<'a> {
    fn from(value: &'a DynamicTuple) -> Self {
        Self::Tuple(value)
    }
}

impl<'a> From<&'a DynamicList> for DynamicValueRef<'a> {
    fn from(value: &'a DynamicList) -> Self {
        Self::List(value)
    }
}

impl<'a> From<&'a DynamicArray> for DynamicValueRef<'a> {
    fn from(value: &'a DynamicArray) -> Self {
        Self::Array(value)
    }
}

impl<'a> From<&'a DynamicMap> for DynamicValueRef<'a> {
    fn from(value: &'a DynamicMap) -> Self {
        Self::Map(value)
    }
}

impl<'a> From<&'a DynamicSet> for DynamicValueRef<'a> {
    fn from(value: &'a DynamicSet) -> Self {
        Self::Set(value)
    }
}

impl<'a> From<&'a DynamicEnum> for DynamicValueRef<'a> {
    fn from(value: &'a DynamicEnum) -> Self {
        Self::Enum(value)
    }
}

impl<'a, T: Reflect> From<&'a T> for DynamicValueRef<'a> {
    fn from(value: &'a T) -> Self {
        Self::Concrete(value)
    }
}

impl<'a> From<&'a dyn Reflect> for DynamicValueRef<'a> {
    fn from(value: &'a dyn Reflect) -> Self {
        Self::Concrete(value)
    }
}

impl<'a> From<&'a Box<dyn Reflect>> for DynamicValueRef<'a> {
    fn from(value: &'a Box<dyn Reflect>) -> Self {
        Self::Concrete(&**value)
    }
}

impl<'a> From<&'a DynamicValueMut<'_>> for DynamicValueRef<'a> {
    fn from(value: &'a DynamicValueMut<'_>) -> Self {
        match value {
            DynamicValueMut::Struct(value) => Self::Struct(value),
            DynamicValueMut::TupleStruct(value) => Self::TupleStruct(value),
            DynamicValueMut::Tuple(value) => Self::Tuple(value),
            DynamicValueMut::List(value) => Self::List(value),
            DynamicValueMut::Array(value) => Self::Array(value),
            DynamicValueMut::Map(value) => Self::Map(value),
            DynamicValueMut::Set(value) => Self::Set(value),
            DynamicValueMut::Enum(value) => Self::Enum(value),
            DynamicValueMut::Concrete(value) => Self::Concrete(&**value),
        }
    }
}

impl<'a> From<DynamicValueMut<'a>> for DynamicValueRef<'a> {
    fn from(value: DynamicValueMut<'a>) -> Self {
        match value {
            DynamicValueMut::Struct(value) => Self::Struct(value),
            DynamicValueMut::TupleStruct(value) => Self::TupleStruct(value),
            DynamicValueMut::Tuple(value) => Self::Tuple(value),
            DynamicValueMut::List(value) => Self::List(value),
            DynamicValueMut::Array(value) => Self::Array(value),
            DynamicValueMut::Map(value) => Self::Map(value),
            DynamicValueMut::Set(value) => Self::Set(value),
            DynamicValueMut::Enum(value) => Self::Enum(value),
            DynamicValueMut::Concrete(value) => Self::Concrete(&*value),
        }
    }
}

/// A dynamically typed mutable reference based on reflection.
///
/// A value can either be a "concrete" [`Reflect`] value, or it may be a dynamic representation such
/// as [`DynamicStruct`].
///
/// This may be a reference into a [`DynamicValue`], but also allows any [`Reflect`] value to be
/// referenced.
pub enum DynamicValueMut<'a> {
    /// A dynamically-typed struct value.
    Struct(&'a mut DynamicStruct),
    /// A dynamically-typed tuple-struct value.
    TupleStruct(&'a mut DynamicTupleStruct),
    /// A dynamically-typed tuple value.
    Tuple(&'a mut DynamicTuple),
    /// A dynamically-typed list value.
    List(&'a mut DynamicList),
    /// A dynamically-typed array value.
    Array(&'a mut DynamicArray),
    /// A dynamically-typed map value.
    Map(&'a mut DynamicMap),
    /// A dynamically-typed set value.
    Set(&'a mut DynamicSet),
    /// A dynamically-typed enum value.
    Enum(&'a mut DynamicEnum),
    /// A concrete reflected value.
    Concrete(&'a mut dyn Reflect),
}

impl<'a> DynamicValueMut<'a> {
    /// Reborrows this dynamic value to have a shorter lifetime.
    pub fn reborrow<'b>(&'b mut self) -> DynamicValueMut<'b> {
        match self {
            Self::Struct(value) => DynamicValueMut::Struct(value),
            Self::TupleStruct(value) => DynamicValueMut::TupleStruct(value),
            Self::Tuple(value) => DynamicValueMut::Tuple(value),
            Self::List(value) => DynamicValueMut::List(value),
            Self::Array(value) => DynamicValueMut::Array(value),
            Self::Map(value) => DynamicValueMut::Map(value),
            Self::Set(value) => DynamicValueMut::Set(value),
            Self::Enum(value) => DynamicValueMut::Enum(value),
            Self::Concrete(value) => DynamicValueMut::Concrete(&mut **value),
        }
    }

    /// Converts this mutable reference into an immutable reference.
    pub fn as_ref<'b>(&'b self) -> DynamicValueRef<'b> {
        DynamicValueRef::from(self)
    }

    /// Consumes this mutable reference and converts it into an immutable reference.
    ///
    /// Unlike [`Self::as_ref`], this method does not shorten the lifetime of the resulting
    /// reference.
    pub fn into_ref(self) -> DynamicValueRef<'a> {
        DynamicValueRef::from(self)
    }

    /// Attempts to clone this value's data.
    ///
    /// Note: Any [`Self::Concrete`] variants first call [`PartialReflect::reflect_clone`], and then
    /// fallback to converting the value to a dynamic representation.
    pub fn dynamic_clone(&self) -> Result<DynamicValue, ReflectCloneError> {
        self.as_ref().dynamic_clone()
    }

    /// Gets the type that this dynamic representation represents.
    pub fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        self.as_ref().get_represented_type_info()
    }

    /// Returns `true` if the underlying value represents a value of type `T`, or `false` otherwise.
    pub fn represents<T: Reflect + TypePath>(&self) -> bool {
        self.as_ref().represents::<T>()
    }

    /// Gets the concrete value if this value is [`Self::Concrete`], returns [`None`] otherwise.
    pub fn as_concrete(&self) -> Option<&dyn Reflect> {
        match self {
            Self::Concrete(value) => Some(&**value),
            _ => None,
        }
    }

    /// Gets the concrete value if this value is [`Self::Concrete`], returns [`None`] otherwise.
    pub fn as_concrete_mut(&mut self) -> Option<&mut dyn Reflect> {
        match self {
            Self::Concrete(value) => Some(&mut **value),
            _ => None,
        }
    }
}

impl<'a> From<&'a mut DynamicValue> for DynamicValueMut<'a> {
    fn from(value: &'a mut DynamicValue) -> Self {
        match value {
            DynamicValue::Struct(value) => Self::Struct(value),
            DynamicValue::TupleStruct(value) => Self::TupleStruct(value),
            DynamicValue::Tuple(value) => Self::Tuple(value),
            DynamicValue::List(value) => Self::List(value),
            DynamicValue::Array(value) => Self::Array(value),
            DynamicValue::Map(value) => Self::Map(value),
            DynamicValue::Set(value) => Self::Set(value),
            DynamicValue::Enum(value) => Self::Enum(value),
            DynamicValue::Concrete(value) => Self::Concrete(&mut **value),
        }
    }
}

impl<'a> From<&'a mut DynamicStruct> for DynamicValueMut<'a> {
    fn from(value: &'a mut DynamicStruct) -> Self {
        Self::Struct(value)
    }
}

impl<'a> From<&'a mut DynamicTupleStruct> for DynamicValueMut<'a> {
    fn from(value: &'a mut DynamicTupleStruct) -> Self {
        Self::TupleStruct(value)
    }
}

impl<'a> From<&'a mut DynamicTuple> for DynamicValueMut<'a> {
    fn from(value: &'a mut DynamicTuple) -> Self {
        Self::Tuple(value)
    }
}

impl<'a> From<&'a mut DynamicList> for DynamicValueMut<'a> {
    fn from(value: &'a mut DynamicList) -> Self {
        Self::List(value)
    }
}

impl<'a> From<&'a mut DynamicArray> for DynamicValueMut<'a> {
    fn from(value: &'a mut DynamicArray) -> Self {
        Self::Array(value)
    }
}

impl<'a> From<&'a mut DynamicMap> for DynamicValueMut<'a> {
    fn from(value: &'a mut DynamicMap) -> Self {
        Self::Map(value)
    }
}

impl<'a> From<&'a mut DynamicSet> for DynamicValueMut<'a> {
    fn from(value: &'a mut DynamicSet) -> Self {
        Self::Set(value)
    }
}

impl<'a> From<&'a mut DynamicEnum> for DynamicValueMut<'a> {
    fn from(value: &'a mut DynamicEnum) -> Self {
        Self::Enum(value)
    }
}

impl<'a, T: Reflect> From<&'a mut T> for DynamicValueMut<'a> {
    fn from(value: &'a mut T) -> Self {
        Self::Concrete(value)
    }
}

impl<'a> From<&'a mut dyn Reflect> for DynamicValueMut<'a> {
    fn from(value: &'a mut dyn Reflect) -> Self {
        Self::Concrete(value)
    }
}

impl<'a> From<&'a mut Box<dyn Reflect>> for DynamicValueMut<'a> {
    fn from(value: &'a mut Box<dyn Reflect>) -> Self {
        Self::Concrete(&mut **value)
    }
}

// This type exists to implement the various reflection traits (like `Struct`, `Array`, etc) on
// `DynamicValue`, while *avoiding* users accidentally using these traits on `DynamicValue`.
#[repr(transparent)]
struct DynamicValueWrapper(DynamicValue);

impl DynamicValueWrapper {
    fn from_ref(value: &DynamicValue) -> &Self {
        #[expect(
            unsafe_code,
            reason = "this allows us to **not** implement PartialReflect for `DynamicValue`, but still use `DynamicValue` as a PartialReflect through this wrapper"
        )]
        // SAFETY: We know that DynamicValueWrapper is repr(transparent), so the values are exactly
        // compatible. There are also no other semanatic invariants for DynamicValueWrapper to
        // uphold.
        unsafe {
            transmute(value)
        }
    }

    fn from_mut(value: &mut DynamicValue) -> &mut Self {
        #[expect(
            unsafe_code,
            reason = "this allows us to **not** implement PartialReflect for `DynamicValue`, but still use `DynamicValue` as a PartialReflect through this wrapper"
        )]
        // SAFETY: We know that DynamicValueWrapper is repr(transparent), so the values are exactly
        // compatible. There are also no other semanatic invariants for DynamicValueWrapper to
        // uphold.
        unsafe {
            transmute(value)
        }
    }

    fn from_box(value: Box<DynamicValue>) -> Box<Self> {
        #[expect(
            unsafe_code,
            reason = "this allows us to **not** implement PartialReflect for `DynamicValue`, but still use `DynamicValue` as a PartialReflect through this wrapper"
        )]
        // SAFETY: We know that DynamicValueWrapper is repr(transparent), so the values are exactly
        // compatible. There are also no other semanatic invariants for DynamicValueWrapper to
        // uphold.
        unsafe {
            transmute(value)
        }
    }
}

impl_type_path!((in bevy_reflect::dynamic) DynamicValueWrapper);

impl PartialReflect for DynamicValueWrapper {
    fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        self.0.get_represented_type_info()
    }

    fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect> {
        self
    }

    fn as_partial_reflect(&self) -> &dyn PartialReflect {
        self
    }

    fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
        self
    }

    fn try_into_reflect(self: Box<Self>) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>> {
        Err(self)
    }

    fn try_as_reflect(&self) -> Option<&dyn Reflect> {
        None
    }

    fn try_as_reflect_mut(&mut self) -> Option<&mut dyn Reflect> {
        None
    }

    fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> {
        match &mut self.0 {
            DynamicValue::Struct(_) => {
                let struct_value = value.reflect_ref().as_struct()?;

                for (name, value) in struct_value {
                    if let Some(v) = <Self as Struct>::field_mut(self, name) {
                        v.try_apply(value)?;
                    }
                }

                Ok(())
            }
            DynamicValue::TupleStruct(_) => {
                let tuple_struct = value.reflect_ref().as_tuple_struct()?;

                for (i, value) in tuple_struct.iter_fields().enumerate() {
                    if let Some(v) = <Self as TupleStruct>::field_mut(self, i) {
                        v.try_apply(value)?;
                    }
                }

                Ok(())
            }
            DynamicValue::Tuple(_) => tuple_try_apply(self, value),
            DynamicValue::List(_) => list_try_apply(self, value),
            DynamicValue::Array(_) => array_try_apply(self, value),
            DynamicValue::Map(_) => map_try_apply(self, value),
            DynamicValue::Set(_) => set_try_apply(self, value),
            DynamicValue::Enum(self_value) => enum_try_apply(self_value, value),
            DynamicValue::Concrete(_) => unreachable!("handled earlier"),
        }
    }

    fn reflect_kind(&self) -> ReflectKind {
        match &self.0 {
            DynamicValue::Struct(_) => ReflectKind::Struct,
            DynamicValue::TupleStruct(_) => ReflectKind::TupleStruct,
            DynamicValue::Tuple(_) => ReflectKind::Tuple,
            DynamicValue::List(_) => ReflectKind::List,
            DynamicValue::Array(_) => ReflectKind::Array,
            DynamicValue::Map(_) => ReflectKind::Map,
            DynamicValue::Set(_) => ReflectKind::Set,
            DynamicValue::Enum(_) => ReflectKind::Enum,
            DynamicValue::Concrete(_) => unreachable!("handled earlier"),
        }
    }

    fn reflect_ref(&self) -> ReflectRef<'_> {
        match &self.0 {
            DynamicValue::Struct(_) => ReflectRef::Struct(self),
            DynamicValue::TupleStruct(_) => ReflectRef::TupleStruct(self),
            DynamicValue::Tuple(_) => ReflectRef::Tuple(self),
            DynamicValue::List(_) => ReflectRef::List(self),
            DynamicValue::Array(_) => ReflectRef::Array(self),
            DynamicValue::Map(_) => ReflectRef::Map(self),
            DynamicValue::Set(_) => ReflectRef::Set(self),
            DynamicValue::Enum(_) => ReflectRef::Enum(self),
            DynamicValue::Concrete(_) => unreachable!("handled earlier"),
        }
    }

    fn reflect_mut(&mut self) -> ReflectMut<'_> {
        match &mut self.0 {
            DynamicValue::Struct(_) => ReflectMut::Struct(self),
            DynamicValue::TupleStruct(_) => ReflectMut::TupleStruct(self),
            DynamicValue::Tuple(_) => ReflectMut::Tuple(self),
            DynamicValue::List(_) => ReflectMut::List(self),
            DynamicValue::Array(_) => ReflectMut::Array(self),
            DynamicValue::Map(_) => ReflectMut::Map(self),
            DynamicValue::Set(_) => ReflectMut::Set(self),
            DynamicValue::Enum(_) => ReflectMut::Enum(self),
            DynamicValue::Concrete(_) => unreachable!("handled earlier"),
        }
    }

    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        match self.0 {
            DynamicValue::Struct(_) => ReflectOwned::Struct(self),
            DynamicValue::TupleStruct(_) => ReflectOwned::TupleStruct(self),
            DynamicValue::Tuple(_) => ReflectOwned::Tuple(self),
            DynamicValue::List(_) => ReflectOwned::List(self),
            DynamicValue::Array(_) => ReflectOwned::Array(self),
            DynamicValue::Map(_) => ReflectOwned::Map(self),
            DynamicValue::Set(_) => ReflectOwned::Set(self),
            DynamicValue::Enum(_) => ReflectOwned::Enum(self),
            // We always bail early if `DynamicValue` stores Concrete, so we never create a wrapper
            // with this variant.
            DynamicValue::Concrete(_) => unreachable!(),
        }
    }

    fn reflect_hash(&self) -> Option<u64> {
        match &self.0 {
            DynamicValue::Struct(_)
            | DynamicValue::TupleStruct(_)
            | DynamicValue::Tuple(_)
            | DynamicValue::Map(_)
            | DynamicValue::Set(_) => None,
            DynamicValue::List(_) => list_hash(self),
            DynamicValue::Array(_) => array_hash(self),
            DynamicValue::Enum(_) => enum_hash(self),
            DynamicValue::Concrete(_) => unreachable!("handled earlier"),
        }
    }

    fn reflect_partial_eq(&self, value: &dyn PartialReflect) -> Option<bool> {
        match &self.0 {
            DynamicValue::Struct(_) => struct_partial_eq(self, value),
            DynamicValue::TupleStruct(_) => tuple_struct_partial_eq(self, value),
            DynamicValue::Tuple(_) => tuple_partial_eq(self, value),
            DynamicValue::List(_) => list_partial_eq(self, value),
            DynamicValue::Array(_) => array_partial_eq(self, value),
            DynamicValue::Map(_) => map_partial_eq(self, value),
            DynamicValue::Set(_) => set_partial_eq(self, value),
            DynamicValue::Enum(_) => enum_partial_eq(self, value),
            DynamicValue::Concrete(_) => unreachable!("handled earlier"),
        }
    }

    fn reflect_partial_cmp(&self, value: &dyn PartialReflect) -> Option<Ordering> {
        match &self.0 {
            DynamicValue::Struct(_) => struct_partial_cmp(self, value),
            DynamicValue::TupleStruct(_) => tuple_struct_partial_cmp(self, value),
            DynamicValue::Tuple(_) => tuple_partial_cmp(self, value),
            DynamicValue::List(_) => list_partial_cmp(self, value),
            DynamicValue::Array(_) => array_partial_cmp(self, value),
            DynamicValue::Map(_) => map_partial_cmp(self, value),
            // Sets don't impl partial_eq.
            DynamicValue::Set(_) => None,
            DynamicValue::Enum(_) => enum_partial_cmp(self, value),
            DynamicValue::Concrete(_) => unreachable!("handled earlier"),
        }
    }

    fn debug(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }

    fn is_dynamic(&self) -> bool {
        !matches!(&self.0, DynamicValue::Concrete(_))
    }
}

impl DynamicValueWrapper {
    fn as_struct(&self) -> &DynamicStruct {
        match &self.0 {
            DynamicValue::Struct(value) => value,
            _ => unreachable!("expected struct"),
        }
    }

    fn as_struct_mut(&mut self) -> &mut DynamicStruct {
        match &mut self.0 {
            DynamicValue::Struct(value) => value,
            _ => unreachable!("expected struct"),
        }
    }

    fn as_tuple_struct(&self) -> &DynamicTupleStruct {
        match &self.0 {
            DynamicValue::TupleStruct(value) => value,
            _ => unreachable!("expected tuple struct"),
        }
    }

    fn as_tuple_struct_mut(&mut self) -> &mut DynamicTupleStruct {
        match &mut self.0 {
            DynamicValue::TupleStruct(value) => value,
            _ => unreachable!("expected tuple struct"),
        }
    }

    fn as_tuple(&self) -> &DynamicTuple {
        match &self.0 {
            DynamicValue::Tuple(value) => value,
            _ => unreachable!("expected tuple"),
        }
    }

    fn as_tuple_mut(&mut self) -> &mut DynamicTuple {
        match &mut self.0 {
            DynamicValue::Tuple(value) => value,
            _ => unreachable!("expected tuple"),
        }
    }

    fn as_tuple_owned(self) -> DynamicTuple {
        match self.0 {
            DynamicValue::Tuple(value) => value,
            _ => unreachable!("expected tuple"),
        }
    }

    fn as_list(&self) -> &DynamicList {
        match &self.0 {
            DynamicValue::List(value) => value,
            _ => unreachable!("expected list"),
        }
    }

    fn as_list_mut(&mut self) -> &mut DynamicList {
        match &mut self.0 {
            DynamicValue::List(value) => value,
            _ => unreachable!("expected list"),
        }
    }

    fn as_array(&self) -> &DynamicArray {
        match &self.0 {
            DynamicValue::Array(value) => value,
            _ => unreachable!("expected array"),
        }
    }

    fn as_array_mut(&mut self) -> &mut DynamicArray {
        match &mut self.0 {
            DynamicValue::Array(value) => value,
            _ => unreachable!("expected array"),
        }
    }

    fn as_array_owned(self) -> DynamicArray {
        match self.0 {
            DynamicValue::Array(value) => value,
            _ => unreachable!("expected array"),
        }
    }

    fn as_map(&self) -> &DynamicMap {
        match &self.0 {
            DynamicValue::Map(value) => value,
            _ => unreachable!("expected map"),
        }
    }

    fn as_map_mut(&mut self) -> &mut DynamicMap {
        match &mut self.0 {
            DynamicValue::Map(value) => value,
            _ => unreachable!("expected map"),
        }
    }

    fn as_set(&self) -> &DynamicSet {
        match &self.0 {
            DynamicValue::Set(value) => value,
            _ => unreachable!("expected set"),
        }
    }

    fn as_set_mut(&mut self) -> &mut DynamicSet {
        match &mut self.0 {
            DynamicValue::Set(value) => value,
            _ => unreachable!("expected set"),
        }
    }

    fn as_enum(&self) -> &DynamicEnum {
        match &self.0 {
            DynamicValue::Enum(value) => value,
            _ => unreachable!("expected enum"),
        }
    }

    fn as_enum_mut(&mut self) -> &mut DynamicEnum {
        match &mut self.0 {
            DynamicValue::Enum(value) => value,
            _ => unreachable!("expected enum"),
        }
    }
}

impl Struct for DynamicValueWrapper {
    fn field(&self, name: &str) -> Option<&dyn PartialReflect> {
        self.as_struct()
            .field(name)
            .map(DynamicValue::as_partial_reflect)
    }

    fn field_mut(&mut self, name: &str) -> Option<&mut dyn PartialReflect> {
        self.as_struct_mut()
            .field_mut(name)
            .map(DynamicValue::as_partial_reflect_mut)
    }

    fn field_at(&self, index: usize) -> Option<&dyn PartialReflect> {
        self.as_struct()
            .field_at(index)
            .map(DynamicValue::as_partial_reflect)
    }

    fn field_at_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect> {
        self.as_struct_mut()
            .field_at_mut(index)
            .map(DynamicValue::as_partial_reflect_mut)
    }

    fn name_at(&self, index: usize) -> Option<&str> {
        self.as_struct().name_at(index)
    }

    fn index_of_name(&self, name: &str) -> Option<usize> {
        self.as_struct().index_of_name(name)
    }

    fn field_len(&self) -> usize {
        self.as_struct().field_len()
    }

    fn iter_fields(&self) -> FieldIter<'_> {
        FieldIter::new(self)
    }

    fn get_represented_struct_info(&self) -> Option<&'static StructInfo> {
        self.as_struct().get_represented_struct_info()
    }
}

impl TupleStruct for DynamicValueWrapper {
    fn field(&self, index: usize) -> Option<&dyn PartialReflect> {
        self.as_tuple_struct()
            .field(index)
            .map(DynamicValue::as_partial_reflect)
    }

    fn field_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect> {
        self.as_tuple_struct_mut()
            .field_mut(index)
            .map(DynamicValue::as_partial_reflect_mut)
    }

    fn field_len(&self) -> usize {
        self.as_tuple_struct().field_len()
    }

    fn iter_fields(&self) -> TupleStructFieldIter<'_> {
        TupleStructFieldIter::new(self)
    }

    fn get_represented_tuple_struct_info(&self) -> Option<&'static TupleStructInfo> {
        self.as_tuple_struct().get_represented_tuple_struct_info()
    }
}

impl Tuple for DynamicValueWrapper {
    fn field(&self, index: usize) -> Option<&dyn PartialReflect> {
        self.as_tuple()
            .field(index)
            .map(DynamicValue::as_partial_reflect)
    }

    fn field_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect> {
        self.as_tuple_mut()
            .field_mut(index)
            .map(DynamicValue::as_partial_reflect_mut)
    }

    fn field_len(&self) -> usize {
        self.as_tuple().field_len()
    }

    fn iter_fields(&self) -> TupleFieldIter<'_> {
        TupleFieldIter::new(self)
    }

    fn drain(self: Box<Self>) -> Vec<Box<dyn PartialReflect>> {
        self.as_tuple_owned()
            .into_fields()
            .into_iter()
            .map(DynamicValue::as_boxed_partial_reflect)
            .collect()
    }

    fn get_represented_tuple_info(&self) -> Option<&'static TupleInfo> {
        self.as_tuple().get_represented_tuple_info()
    }
}

impl List for DynamicValueWrapper {
    fn get(&self, index: usize) -> Option<&dyn PartialReflect> {
        self.as_list()
            .get(index)
            .map(DynamicValue::as_partial_reflect)
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect> {
        self.as_list_mut()
            .get_mut(index)
            .map(DynamicValue::as_partial_reflect_mut)
    }

    fn value_from_dynamic(&self, dynamic: DynamicValue) -> Option<Box<dyn PartialReflect>> {
        Some(dynamic.as_boxed_partial_reflect())
    }

    fn insert(&mut self, index: usize, element: Box<dyn PartialReflect>) {
        // TODO: Don't unwrap here.
        self.as_list_mut()
            .insert(index, DynamicValue::try_from_reflect(&*element).unwrap());
    }

    fn remove(&mut self, index: usize) -> Box<dyn PartialReflect> {
        self.as_list_mut().remove(index).as_boxed_partial_reflect()
    }

    fn swap(&mut self, a: usize, b: usize) {
        self.as_list_mut().swap(a, b);
    }

    fn len(&self) -> usize {
        self.as_list().len()
    }

    fn iter(&self) -> ListIter<'_> {
        ListIter::new(self)
    }

    fn drain(&mut self) -> Vec<Box<dyn PartialReflect>> {
        self.as_list_mut()
            .drain()
            .into_iter()
            .map(DynamicValue::as_boxed_partial_reflect)
            .collect()
    }

    fn push(&mut self, value: Box<dyn PartialReflect>) {
        // TODO: Don't unwrap here.
        self.as_list_mut()
            .push(DynamicValue::try_from_reflect(&*value).unwrap());
    }

    fn pop(&mut self) -> Option<Box<dyn PartialReflect>> {
        self.as_list_mut()
            .pop()
            .map(DynamicValue::as_boxed_partial_reflect)
    }

    fn is_empty(&self) -> bool {
        self.as_list().is_empty()
    }

    fn get_represented_list_info(&self) -> Option<&'static ListInfo> {
        self.as_list().get_represented_list_info()
    }
}

impl Array for DynamicValueWrapper {
    fn get(&self, index: usize) -> Option<&dyn PartialReflect> {
        self.as_array()
            .get(index)
            .map(DynamicValue::as_partial_reflect)
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect> {
        self.as_array_mut()
            .get_mut(index)
            .map(DynamicValue::as_partial_reflect_mut)
    }

    fn swap(&mut self, a: usize, b: usize) {
        self.as_array_mut().swap(a, b);
    }

    fn len(&self) -> usize {
        self.as_array().len()
    }

    fn iter(&self) -> ArrayIter<'_> {
        ArrayIter::new(self)
    }

    fn drain(self: Box<Self>) -> Vec<Box<dyn PartialReflect>> {
        self.as_array_owned()
            .into_values()
            .into_iter()
            .map(DynamicValue::as_boxed_partial_reflect)
            .collect()
    }

    fn is_empty(&self) -> bool {
        self.as_array().is_empty()
    }

    fn get_represented_array_info(&self) -> Option<&'static ArrayInfo> {
        self.as_array().get_represented_array_info()
    }
}

impl Map for DynamicValueWrapper {
    fn key_from_dynamic(&self, dynamic: DynamicValue) -> Option<Box<dyn PartialReflect>> {
        Some(dynamic.as_boxed_partial_reflect())
    }

    fn value_from_dynamic(&self, dynamic: DynamicValue) -> Option<Box<dyn PartialReflect>> {
        Some(dynamic.as_boxed_partial_reflect())
    }

    fn get(&self, key: &dyn PartialReflect) -> Option<&dyn PartialReflect> {
        // TODO: Don't unwrap here.
        self.as_map()
            .get(DynamicValue::try_from_reflect(key).unwrap().as_ref())
            .map(DynamicValue::as_partial_reflect)
    }

    fn get_mut(&mut self, key: &dyn PartialReflect) -> Option<&mut dyn PartialReflect> {
        // TODO: Don't unwrap here.
        self.as_map_mut()
            .get_mut(DynamicValue::try_from_reflect(key).unwrap().as_ref())
            .map(DynamicValue::as_partial_reflect_mut)
    }

    fn len(&self) -> usize {
        self.as_map().len()
    }

    fn iter(&self) -> Box<dyn Iterator<Item = (&dyn PartialReflect, &dyn PartialReflect)> + '_> {
        Box::new(
            self.as_map()
                .iter()
                .map(|(k, v)| (k.as_partial_reflect(), v.as_partial_reflect())),
        )
    }

    fn drain(&mut self) -> Vec<(Box<dyn PartialReflect>, Box<dyn PartialReflect>)> {
        self.as_map_mut().drain()
    }

    fn retain(&mut self, f: &mut dyn FnMut(&dyn PartialReflect, &mut dyn PartialReflect) -> bool) {
        self.as_map_mut()
            .retain(&mut move |k, v| f(k.as_partial_reflect(), v.as_partial_reflect_mut()));
    }

    fn insert_boxed(
        &mut self,
        key: Box<dyn PartialReflect>,
        value: Box<dyn PartialReflect>,
    ) -> Option<Box<dyn PartialReflect>> {
        // TODO: Don't unwrap here.
        let key = DynamicValue::try_from_reflect(&*key).unwrap();
        let value = DynamicValue::try_from_reflect(&*value).unwrap();
        self.as_map_mut()
            .insert(key, value)
            .map(DynamicValue::as_boxed_partial_reflect)
    }

    fn remove(&mut self, key: &dyn PartialReflect) -> Option<Box<dyn PartialReflect>> {
        // TODO: Don't unwrap here.
        let key = DynamicValue::try_from_reflect(&*key).unwrap();
        self.as_map_mut()
            .remove(key.as_ref())
            .map(DynamicValue::as_boxed_partial_reflect)
    }

    fn is_empty(&self) -> bool {
        self.as_map().is_empty()
    }

    fn get_represented_map_info(&self) -> Option<&'static MapInfo> {
        self.as_map().get_represented_map_info()
    }
}

impl Set for DynamicValueWrapper {
    fn value_from_dynamic(&self, dynamic: DynamicValue) -> Option<Box<dyn PartialReflect>> {
        Some(dynamic.as_boxed_partial_reflect())
    }

    fn get(&self, value: &dyn PartialReflect) -> Option<&dyn PartialReflect> {
        // TODO: Don't unwrap here.
        let value = DynamicValue::try_from_reflect(value).unwrap();
        self.as_set()
            .get(value.as_ref())
            .map(DynamicValue::as_partial_reflect)
    }

    fn len(&self) -> usize {
        self.as_set().len()
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &dyn PartialReflect> + '_> {
        Box::new(self.as_set().iter().map(DynamicValue::as_partial_reflect))
    }

    fn drain(&mut self) -> Vec<Box<dyn PartialReflect>> {
        self.as_set_mut()
            .drain()
            .into_iter()
            .map(DynamicValue::as_boxed_partial_reflect)
            .collect()
    }

    fn retain(&mut self, f: &mut dyn FnMut(&dyn PartialReflect) -> bool) {
        self.as_set_mut()
            .retain(&mut move |value| f(value.as_partial_reflect()));
    }

    fn insert_boxed(&mut self, value: Box<dyn PartialReflect>) -> bool {
        // TODO: Don't unwrap here.
        let value = DynamicValue::try_from_reflect(&*value).unwrap();
        self.as_set_mut().insert(value)
    }

    fn remove(&mut self, value: &dyn PartialReflect) -> bool {
        // TODO: Don't unwrap here.
        let value = DynamicValue::try_from_reflect(&*value).unwrap();
        self.as_set_mut().remove(value.as_ref())
    }

    fn contains(&self, value: &dyn PartialReflect) -> bool {
        // TODO: Don't unwrap here.
        let value = DynamicValue::try_from_reflect(&*value).unwrap();
        self.as_set().contains(value.as_ref())
    }

    fn is_empty(&self) -> bool {
        self.as_set().is_empty()
    }
}

impl Enum for DynamicValueWrapper {
    fn set_from_dynamic(&mut self, dynamic: DynamicEnum) -> Result<(), ApplyError> {
        self.0 = DynamicValue::Enum(dynamic);
        Ok(())
    }

    fn field(&self, name: &str) -> Option<&dyn PartialReflect> {
        self.as_enum()
            .field(name)
            .map(DynamicValue::as_partial_reflect)
    }

    fn field_at(&self, index: usize) -> Option<&dyn PartialReflect> {
        self.as_enum()
            .field_at(index)
            .map(DynamicValue::as_partial_reflect)
    }

    fn field_mut(&mut self, name: &str) -> Option<&mut dyn PartialReflect> {
        self.as_enum_mut()
            .field_mut(name)
            .map(DynamicValue::as_partial_reflect_mut)
    }

    fn field_at_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect> {
        self.as_enum_mut()
            .field_at_mut(index)
            .map(DynamicValue::as_partial_reflect_mut)
    }

    fn index_of(&self, name: &str) -> Option<usize> {
        self.as_enum().index_of(name)
    }

    fn name_at(&self, index: usize) -> Option<&str> {
        self.as_enum().name_at(index)
    }

    fn iter_fields(&self) -> VariantFieldIter<'_> {
        VariantFieldIter::new(self)
    }

    fn field_len(&self) -> usize {
        self.as_enum().field_len()
    }

    fn variant_name(&self) -> &str {
        self.as_enum().variant_name()
    }

    fn variant_index(&self) -> usize {
        self.as_enum().variant_index()
    }

    fn variant_type(&self) -> crate::enums::VariantType {
        self.as_enum().variant_type()
    }

    fn is_variant(&self, variant_type: crate::enums::VariantType) -> bool {
        self.as_enum().variant_type() == variant_type
    }

    fn variant_path(&self) -> String {
        self.as_enum().variant_path()
    }

    fn get_represented_enum_info(&self) -> Option<&'static crate::enums::EnumInfo> {
        self.as_enum().get_represented_enum_info()
    }
}

/// A struct type which allows fields to be added at runtime.
#[derive(Default)]
pub struct DynamicStruct {
    represented_type: Option<&'static TypeInfo>,
    fields: Vec<DynamicValue>,
    field_names: Vec<Cow<'static, str>>,
    field_indices: HashMap<Cow<'static, str>, usize>,
}

impl DynamicStruct {
    /// Creates an equivalent dynamic representation of `value`.
    ///
    /// This function clones opaque fields out of the original `value`, which can fail if a field
    /// does not implement [`Clone`].
    pub fn try_from_reflect(value: &dyn Struct) -> Result<Self, ReflectCloneError> {
        let represented_type = value.get_represented_type_info();
        let mut fields = Vec::with_capacity(value.field_len());
        let mut field_names = Vec::with_capacity(value.field_len());
        let mut field_indices = HashMap::new();
        for (index, (name, value)) in value.iter_fields().enumerate() {
            fields.push(DynamicValue::try_from_reflect(value)?);
            field_names.push(name.to_string().into());
            field_indices.insert(name.to_string().into(), index);
        }

        Ok(Self {
            represented_type,
            fields,
            field_names,
            field_indices,
        })
    }

    /// Apply the values in `self` to the provided `reflect` using reflection.
    ///
    /// If this function fails, the `reflect` may be partially mutated.
    pub fn try_apply(self, reflect: &mut dyn Struct) -> Result<(), ApplyError> {
        for (name, value) in self.field_names.into_iter().zip(self.fields.into_iter()) {
            let Some(field) = reflect.field_mut(&name) else {
                // TODO: Should this be an error?
                continue;
            };
            // This unwrap should be safe since the original value was `Reflect`, so all of
            // its fields should also be `Reflect`.
            let field = field.try_as_reflect_mut().unwrap();
            value.try_apply(field)?;
        }
        Ok(())
    }

    /// Attempts to clone this value's data.
    ///
    /// May fail if an opaque value cannot be cloned.
    pub fn dynamic_clone(&self) -> Result<Self, ReflectCloneError> {
        Ok(Self {
            represented_type: self.represented_type,
            fields: self
                .fields
                .iter()
                .map(DynamicValue::dynamic_clone)
                .collect::<Result<_, _>>()?,
            field_names: self.field_names.clone(),
            field_indices: self.field_indices.clone(),
        })
    }

    /// Sets the [type] to be represented by this `DynamicStruct`.
    ///
    /// # Panics
    ///
    /// Panics if the given [type] is not a [`TypeInfo::Struct`].
    ///
    /// [type]: TypeInfo
    pub fn set_represented_type(&mut self, represented_type: Option<&'static TypeInfo>) {
        if let Some(represented_type) = represented_type {
            assert!(
                matches!(represented_type, TypeInfo::Struct(_)),
                "expected TypeInfo::Struct but received: {represented_type:?}"
            );
        }

        self.represented_type = represented_type;
    }

    /// Inserts a field named `name` with value `value` into the struct.
    ///
    /// If the field already exists, it is overwritten.
    pub fn insert<'a>(&mut self, name: impl Into<Cow<'a, str>>, field: impl Into<DynamicValue>) {
        let name: Cow<str> = name.into();
        let field = field.into();
        if let Some(index) = self.field_indices.get(&name) {
            self.fields[*index] = field;
        } else {
            self.fields.push(field);
            self.field_indices
                .insert(Cow::Owned(name.clone().into_owned()), self.fields.len() - 1);
            self.field_names.push(Cow::Owned(name.into_owned()));
        }
    }

    /// Removes a field at `index`.
    pub fn remove_at(
        &mut self,
        index: usize,
    ) -> Option<(Cow<'static, str>, Box<dyn PartialReflect>)> {
        let mut i: usize = 0;
        let mut extract = self.field_names.extract_if(0..self.field_names.len(), |n| {
            let mut result = false;
            if i == index {
                self.field_indices
                    .remove(n)
                    .expect("Invalid name for `field_indices.remove(name)`");
                result = true;
            } else if i > index {
                *self
                    .field_indices
                    .get_mut(n)
                    .expect("Invalid name for `field_indices.get_mut(name)`") -= 1;
            }
            i += 1;
            result
        });

        let name = extract
            .nth(0)
            .expect("Invalid index for `extract.nth(index)`");
        extract.for_each(drop); // Fully evaluate the rest of the iterator, so we don't short-circuit the extract.

        Some((name, self.fields.remove(index).as_boxed_partial_reflect()))
    }

    /// Removes the first field that satisfies the given predicate, `f`.
    pub fn remove_if<F>(&mut self, mut f: F) -> Option<(Cow<'static, str>, Box<dyn PartialReflect>)>
    where
        F: FnMut((&str, &dyn PartialReflect)) -> bool,
    {
        if let Some(index) = self
            .field_names
            .deref()
            .iter()
            .zip(self.fields.iter())
            .position(|(name, field)| f((name.as_ref(), field.as_partial_reflect())))
        {
            self.remove_at(index)
        } else {
            None
        }
    }

    /// Removes a field by `name`.
    pub fn remove_by_name(
        &mut self,
        name: &str,
    ) -> Option<(Cow<'static, str>, Box<dyn PartialReflect>)> {
        if let Some(index) = self.index_of_name(name) {
            self.remove_at(index)
        } else {
            None
        }
    }

    /// Gets the value of a field by the field's name.
    ///
    /// Returns [`None`] if there is no field with such a name.
    #[inline]
    pub fn field(&self, name: &str) -> Option<&DynamicValue> {
        self.field_indices
            .get(name)
            .map(|index| &self.fields[*index])
    }

    /// Gets the value mutably of a field by the field's name.
    ///
    /// Returns [`None`] if there is no field with such a name.
    #[inline]
    pub fn field_mut(&mut self, name: &str) -> Option<&mut DynamicValue> {
        if let Some(index) = self.index_of_name(name) {
            Some(&mut self.fields[index])
        } else {
            None
        }
    }

    /// Gets the value of a field by the field's index.
    ///
    /// Returns [`None`] if the field index is invalid.
    #[inline]
    pub fn field_at(&self, index: usize) -> Option<&DynamicValue> {
        self.fields.get(index)
    }

    /// Gets the value mutably of a field by the field's index.
    ///
    /// Returns [`None`] if the field index is invalid.
    #[inline]
    pub fn field_at_mut(&mut self, index: usize) -> Option<&mut DynamicValue> {
        self.fields.get_mut(index)
    }

    /// Gets the name of a field by the field's index.
    ///
    /// Returns [`None`] if the field index is invalid.
    #[inline]
    pub fn name_at(&self, index: usize) -> Option<&str> {
        self.field_names.deref().get(index).map(AsRef::as_ref)
    }

    /// Gets the index of the field with the given name.
    ///
    /// Returns [`None`] if there is no field with the given name.
    #[inline]
    pub fn index_of_name(&self, name: &str) -> Option<usize> {
        self.field_indices.get(name).copied()
    }

    /// Gets the number of fields in this struct.
    #[inline]
    pub fn field_len(&self) -> usize {
        self.fields.len()
    }

    /// Returns an iterator of all the fields with their name and value.
    #[inline]
    pub fn iter_fields(&self) -> impl Iterator<Item = (&str, &DynamicValue)> + ExactSizeIterator {
        self.field_names
            .deref()
            .iter()
            .map(AsRef::as_ref)
            .zip(self.fields.iter())
    }

    /// Gets the type info of the represented struct, if this dynamic value represents a known type.
    pub fn get_represented_struct_info(&self) -> Option<&'static StructInfo> {
        self.represented_type?.as_struct().ok()
    }
}

impl Debug for DynamicStruct {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "DynamicStruct(")?;
        let mut debug = f.debug_struct(
            self.represented_type
                .map(TypeInfo::type_path)
                .unwrap_or("_"),
        );
        for (field, name) in self.fields.iter().zip(self.field_names.deref().iter()) {
            debug.field(name, field as &dyn Debug);
        }
        debug.finish()?;
        write!(f, ")")
    }
}

impl_type_path!((in bevy_reflect::dynamic) DynamicStruct);

impl<'a, N, V> FromIterator<(N, V)> for DynamicStruct
where
    N: Into<Cow<'a, str>>,
    V: Into<DynamicValue>,
{
    /// Create a dynamic struct that doesn't represent a type from the
    /// field name, field value pairs.
    fn from_iter<I: IntoIterator<Item = (N, V)>>(fields: I) -> Self {
        let mut dynamic_struct = Self::default();
        for (name, value) in fields.into_iter() {
            dynamic_struct.insert(name, value);
        }
        dynamic_struct
    }
}

impl IntoIterator for DynamicStruct {
    type Item = (Cow<'static, str>, DynamicValue);
    type IntoIter = core::iter::Zip<
        alloc::vec::IntoIter<Cow<'static, str>>,
        alloc::vec::IntoIter<DynamicValue>,
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.field_names.into_iter().zip(self.fields.into_iter())
    }
}

/// A tuple struct which allows fields to be added at runtime.
#[derive(Default)]
pub struct DynamicTupleStruct {
    pub(crate) represented_type: Option<&'static TypeInfo>,
    fields: Vec<DynamicValue>,
}

impl DynamicTupleStruct {
    /// Creates an equivalent dynamic representation of `value`.
    ///
    /// This function clones opaque fields out of the original `value`, which can fail if a field
    /// does not implement [`Clone`].
    pub fn try_from_reflect(value: &dyn TupleStruct) -> Result<Self, ReflectCloneError> {
        let represented_type = value.get_represented_type_info();

        let mut fields = Vec::with_capacity(value.field_len());
        for field in value.iter_fields() {
            fields.push(DynamicValue::try_from_reflect(field)?);
        }

        Ok(Self {
            represented_type,
            fields,
        })
    }

    /// Apply the values in `self` to the provided `reflect` using reflection.
    ///
    /// If this function fails, the `reflect` may be partially mutated.
    pub fn try_apply(self, reflect: &mut dyn TupleStruct) -> Result<(), ApplyError> {
        for (index, value) in self.fields.into_iter().enumerate() {
            let Some(field) = reflect.field_mut(index) else {
                // TODO: Should this be an error?
                continue;
            };
            // This unwrap should be safe since the original value was `Reflect`, so all of
            // its fields should also be `Reflect`.
            let field = field.try_as_reflect_mut().unwrap();
            value.try_apply(field)?;
        }
        Ok(())
    }

    /// Attempts to clone this value's data.
    ///
    /// May fail if an opaque value cannot be cloned.
    pub fn dynamic_clone(&self) -> Result<Self, ReflectCloneError> {
        Ok(Self {
            represented_type: self.represented_type,
            fields: self
                .fields
                .iter()
                .map(DynamicValue::dynamic_clone)
                .collect::<Result<_, _>>()?,
        })
    }

    /// Sets the [type] to be represented by this `DynamicTupleStruct`.
    ///
    /// # Panics
    ///
    /// Panics if the given [type] is not a [`TypeInfo::TupleStruct`].
    ///
    /// [type]: TypeInfo
    pub fn set_represented_type(&mut self, represented_type: Option<&'static TypeInfo>) {
        if let Some(represented_type) = represented_type {
            assert!(
                matches!(represented_type, TypeInfo::TupleStruct(_)),
                "expected TypeInfo::TupleStruct but received: {represented_type:?}"
            );
        }

        self.represented_type = represented_type;
    }

    /// Appends a typed element with value `value` to the tuple struct.
    pub fn insert(&mut self, value: impl Into<DynamicValue>) {
        self.fields.push(value.into());
    }

    /// Gets the value of a field by the field's index.
    ///
    /// Returns [`None`] if the field index is invalid.
    #[inline]
    pub fn field(&self, index: usize) -> Option<&DynamicValue> {
        self.fields.get(index)
    }

    /// Gets the value mutably of a field by the field's index.
    ///
    /// Returns [`None`] if the field index is invalid.
    #[inline]
    pub fn field_mut(&mut self, index: usize) -> Option<&mut DynamicValue> {
        self.fields.get_mut(index)
    }

    /// Gets the number of fields in this tuple struct.
    #[inline]
    pub fn field_len(&self) -> usize {
        self.fields.len()
    }

    /// Returns an iterator of all the field values.
    #[inline]
    pub fn iter_fields(&self) -> impl Iterator<Item = &DynamicValue> + ExactSizeIterator {
        self.fields.iter()
    }

    /// Gets the type info of the represented tuple struct, if this dynamic value represents a known
    /// type.
    pub fn get_represented_tuple_struct_info(&self) -> Option<&'static TupleStructInfo> {
        self.represented_type?.as_tuple_struct().ok()
    }
}

impl Debug for DynamicTupleStruct {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "DynamicTupleStruct(")?;
        let mut debug = f.debug_tuple(
            self.represented_type
                .map(TypeInfo::type_path)
                .unwrap_or("_"),
        );
        for field in self.fields.iter() {
            debug.field(field as &dyn Debug);
        }
        debug.finish()?;
        write!(f, ")")
    }
}

impl_type_path!((in bevy_reflect::dynamic) DynamicTupleStruct);

impl From<DynamicTuple> for DynamicTupleStruct {
    fn from(value: DynamicTuple) -> Self {
        Self {
            represented_type: None,
            fields: value.fields,
        }
    }
}

impl<T: Into<DynamicValue>> FromIterator<T> for DynamicTupleStruct {
    fn from_iter<I: IntoIterator<Item = T>>(fields: I) -> Self {
        Self {
            represented_type: None,
            fields: fields.into_iter().map(Into::into).collect(),
        }
    }
}

impl IntoIterator for DynamicTupleStruct {
    type Item = DynamicValue;
    type IntoIter = alloc::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.fields.into_iter()
    }
}

/// A tuple which allows fields to be added at runtime.
#[derive(Default)]
pub struct DynamicTuple {
    pub(crate) represented_type: Option<&'static TypeInfo>,
    pub(crate) fields: Vec<DynamicValue>,
}

impl DynamicTuple {
    /// Creates an equivalent dynamic representation of `value`.
    ///
    /// This function clones opaque fields out of the original `value`, which can fail if a field
    /// does not implement [`Clone`].
    pub fn try_from_reflect(value: &dyn Tuple) -> Result<Self, ReflectCloneError> {
        let represented_type = value.get_represented_type_info();

        let mut fields = Vec::with_capacity(value.field_len());
        for field in value.iter_fields() {
            fields.push(DynamicValue::try_from_reflect(field)?);
        }

        Ok(Self {
            represented_type,
            fields,
        })
    }

    /// Apply the values in `self` to the provided `reflect` using reflection.
    ///
    /// If this function fails, the `reflect` may be partially mutated.
    pub fn try_apply(self, reflect: &mut dyn Tuple) -> Result<(), ApplyError> {
        for (index, value) in self.fields.into_iter().enumerate() {
            let Some(field) = reflect.field_mut(index) else {
                // TODO: Should this be an error?
                continue;
            };
            // This unwrap should be safe since the original value was `Reflect`, so all of
            // its fields should also be `Reflect`.
            let field = field.try_as_reflect_mut().unwrap();
            value.try_apply(field)?;
        }
        Ok(())
    }

    /// Attempts to clone this value's data.
    ///
    /// May fail if an opaque value cannot be cloned.
    pub fn dynamic_clone(&self) -> Result<Self, ReflectCloneError> {
        Ok(Self {
            represented_type: self.represented_type,
            fields: self
                .fields
                .iter()
                .map(DynamicValue::dynamic_clone)
                .collect::<Result<_, _>>()?,
        })
    }

    /// Sets the [type] to be represented by this `DynamicTuple`.
    ///
    /// # Panics
    ///
    /// Panics if the given [type] is not a [`TypeInfo::Tuple`].
    ///
    /// [type]: TypeInfo
    pub fn set_represented_type(&mut self, represented_type: Option<&'static TypeInfo>) {
        if let Some(represented_type) = represented_type {
            assert!(
                matches!(represented_type, TypeInfo::Tuple(_)),
                "expected TypeInfo::Tuple but received: {represented_type:?}"
            );
        }
        self.represented_type = represented_type;
    }

    /// Appends an element with value `value` to the tuple.
    pub fn insert(&mut self, value: impl Into<DynamicValue>) {
        self.represented_type = None;
        self.fields.push(value.into());
    }

    /// Takes the fields that make up this dynamic tuple.
    pub fn into_fields(self) -> Vec<DynamicValue> {
        self.fields
    }

    /// Gets the value of a field by the field's index.
    ///
    /// Returns [`None`] if the field index is invalid.
    #[inline]
    pub fn field(&self, index: usize) -> Option<&DynamicValue> {
        self.fields.get(index)
    }

    /// Gets the value mutably of a field by the field's index.
    ///
    /// Returns [`None`] if the field index is invalid.
    #[inline]
    pub fn field_mut(&mut self, index: usize) -> Option<&mut DynamicValue> {
        self.fields.get_mut(index)
    }

    /// Gets the number of fields in this tuple.
    #[inline]
    pub fn field_len(&self) -> usize {
        self.fields.len()
    }

    /// Returns an iterator of all the field values.
    #[inline]
    pub fn iter_fields(&self) -> impl Iterator<Item = &DynamicValue> + ExactSizeIterator {
        self.fields.iter()
    }

    /// Gets the type info of the represented tuple, if this dynamic value represents a known type.
    pub fn get_represented_tuple_info(&self) -> Option<&'static TupleInfo> {
        self.represented_type?.as_tuple().ok()
    }
}

impl Debug for DynamicTuple {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "DynamicTuple(")?;
        let mut debug = f.debug_tuple("");
        for field in self.fields.iter() {
            debug.field(field as &dyn Debug);
        }
        debug.finish()?;
        write!(f, ")")
    }
}

impl_type_path!((in bevy_reflect) DynamicTuple);

impl<T: Into<DynamicValue>> FromIterator<T> for DynamicTuple {
    fn from_iter<I: IntoIterator<Item = T>>(fields: I) -> Self {
        Self {
            represented_type: None,
            fields: fields.into_iter().map(Into::into).collect(),
        }
    }
}

impl IntoIterator for DynamicTuple {
    type Item = DynamicValue;
    type IntoIter = alloc::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.fields.into_iter()
    }
}

/// A list of reflected values.
#[derive(Default)]
pub struct DynamicList {
    pub(crate) represented_type: Option<&'static TypeInfo>,
    values: Vec<DynamicValue>,
}

impl DynamicList {
    /// Creates an equivalent dynamic representation of `value`.
    ///
    /// This function clones opaque fields out of the original `value`, which can fail if a field
    /// does not implement [`Clone`].
    pub fn try_from_reflect(value: &dyn List) -> Result<Self, ReflectCloneError> {
        let represented_type = value.get_represented_type_info();

        let mut values = Vec::with_capacity(value.len());
        for value in value.iter() {
            values.push(DynamicValue::try_from_reflect(value)?);
        }

        Ok(Self {
            represented_type,
            values,
        })
    }

    /// Apply the values in `self` to the provided `reflect` using reflection.
    ///
    /// If this function fails, the `reflect` may be partially mutated.
    pub fn try_apply(self, reflect: &mut dyn List) -> Result<(), ApplyError> {
        for (index, value) in self.values.into_iter().enumerate() {
            if index < reflect.len() {
                let Some(reflect_value) = reflect.get_mut(index) else {
                    // TODO: This should be an error. Their list impl is bad.
                    continue;
                };
                // This unwrap should be safe since the original value was `Reflect`, so all of
                // its fields should also be `Reflect`.
                let reflect_value = reflect_value.try_as_reflect_mut().unwrap();
                value.try_apply(reflect_value)?;
            } else {
                let value = match reflect.value_from_dynamic(value) {
                    None => return Err(ApplyError::InvalidDynamicValueForCollection),
                    Some(value) => value,
                };
                reflect.push(value);
            }
        }
        Ok(())
    }

    /// Attempts to clone this value's data.
    ///
    /// May fail if an opaque value cannot be cloned.
    pub fn dynamic_clone(&self) -> Result<Self, ReflectCloneError> {
        Ok(Self {
            represented_type: self.represented_type,
            values: self
                .values
                .iter()
                .map(DynamicValue::dynamic_clone)
                .collect::<Result<_, _>>()?,
        })
    }

    /// Sets the [type] to be represented by this `DynamicList`.
    /// # Panics
    ///
    /// Panics if the given [type] is not a [`TypeInfo::List`].
    ///
    /// [type]: TypeInfo
    pub fn set_represented_type(&mut self, represented_type: Option<&'static TypeInfo>) {
        if let Some(represented_type) = represented_type {
            assert!(
                matches!(represented_type, TypeInfo::List(_)),
                "expected TypeInfo::List but received: {represented_type:?}"
            );
        }

        self.represented_type = represented_type;
    }

    /// Appends a typed value to the list.
    pub fn push(&mut self, value: impl Into<DynamicValue>) {
        self.values.push(value.into());
    }

    /// Gets the value of the element at the given index.
    ///
    /// Returns [`None`] if the index is invalid.
    pub fn get(&self, index: usize) -> Option<&DynamicValue> {
        self.values.get(index)
    }

    /// Gets the value mutably of the element at the given index.
    ///
    /// Returns [`None`] if the index is invalid.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut DynamicValue> {
        self.values.get_mut(index)
    }

    /// Inserts a dynamic value at the given index, pushing elements back if needed.
    pub fn insert(&mut self, index: usize, element: DynamicValue) {
        self.values.insert(index, element);
    }

    /// Removes a value from this list and returns it.
    ///
    /// Panics if the index is out of range.
    pub fn remove(&mut self, index: usize) -> DynamicValue {
        self.values.remove(index)
    }

    /// Removes the last element in this list and returns it.
    ///
    /// Returns [`None`] if there are no elements in the list.
    pub fn pop(&mut self) -> Option<DynamicValue> {
        self.values.pop()
    }

    /// Returns the number of elements in the list.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns whether the list is empty.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Returns an iterator over the elements of this list.
    pub fn iter(&self) -> impl Iterator<Item = &DynamicValue> + ExactSizeIterator {
        self.values.iter()
    }

    /// Removes all elements, returning the values as a [`Vec`].
    pub fn drain(&mut self) -> Vec<DynamicValue> {
        core::mem::take(&mut self.values)
    }

    /// Swaps two elements by their index.
    ///
    /// Panics if `a` or `b` are out of bounds.
    pub fn swap(&mut self, a: usize, b: usize) {
        self.values.swap(a, b);
    }

    /// Gets the type info of the represented list, if this dynamic value represents a known type.
    pub fn get_represented_list_info(&self) -> Option<&'static ListInfo> {
        self.represented_type?.as_list().ok()
    }
}

impl Debug for DynamicList {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "DynamicList(")?;
        let mut debug = f.debug_list();
        for item in self.values.iter() {
            debug.entry(item as &dyn Debug);
        }
        debug.finish()?;
        write!(f, ")")
    }
}

impl_type_path!((in bevy_reflect) DynamicList);

impl<T: Into<DynamicValue>> FromIterator<T> for DynamicList {
    fn from_iter<I: IntoIterator<Item = T>>(values: I) -> Self {
        Self {
            represented_type: None,
            values: values.into_iter().map(Into::into).collect(),
        }
    }
}

impl IntoIterator for DynamicList {
    type Item = DynamicValue;
    type IntoIter = alloc::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

/// A fixed-size list of reflected values.
///
/// This differs from [`DynamicList`] in that the size of the [`DynamicArray`]
/// is constant, whereas a [`DynamicList`] can have items added and removed.
///
/// This isn't to say that a [`DynamicArray`] is immutable— its items
/// can be mutated— just that the _number_ of items cannot change.
///
/// [`DynamicList`]: crate::dynamic::DynamicList
pub struct DynamicArray {
    pub(crate) represented_type: Option<&'static TypeInfo>,
    pub(crate) values: Box<[DynamicValue]>,
}

impl DynamicArray {
    /// Creates an equivalent dynamic representation of `value`.
    ///
    /// This function clones opaque fields out of the original `value`, which can fail if a field
    /// does not implement [`Clone`].
    pub fn try_from_reflect(value: &dyn Array) -> Result<Self, ReflectCloneError> {
        let represented_type = value.get_represented_type_info();

        let mut values = Vec::with_capacity(value.len());
        for value in value.iter() {
            values.push(DynamicValue::try_from_reflect(value)?);
        }

        Ok(Self {
            represented_type,
            values: values.into_boxed_slice(),
        })
    }

    /// Apply the values in `self` to the provided `reflect` using reflection.
    ///
    /// If this function fails, the `reflect` may be partially mutated.
    pub fn try_apply(self, reflect: &mut dyn Array) -> Result<(), ApplyError> {
        if self.len() != reflect.len() {
            return Err(ApplyError::DifferentSize {
                from_size: self.len(),
                to_size: reflect.len(),
            });
        }
        for (index, value) in self.values.into_iter().enumerate() {
            let reflect_value = reflect.get_mut(index).unwrap();
            // This unwrap should be safe since the original value was `Reflect`, so all of
            // its values should also be `Reflect`.
            let reflect_value = reflect_value.try_as_reflect_mut().unwrap();
            value.try_apply(reflect_value)?;
        }
        Ok(())
    }

    /// Attempts to clone this value's data.
    ///
    /// May fail if an opaque value cannot be cloned.
    pub fn dynamic_clone(&self) -> Result<Self, ReflectCloneError> {
        Ok(Self {
            represented_type: self.represented_type,
            values: self
                .values
                .iter()
                .map(DynamicValue::dynamic_clone)
                .collect::<Result<_, _>>()?,
        })
    }

    /// Creates a new [`DynamicArray`].
    #[inline]
    pub fn new(values: Box<[DynamicValue]>) -> Self {
        Self {
            represented_type: None,
            values,
        }
    }

    /// Sets the [type] to be represented by this `DynamicArray`.
    ///
    /// # Panics
    ///
    /// Panics if the given [type] is not a [`TypeInfo::Array`].
    ///
    /// [type]: TypeInfo
    pub fn set_represented_type(&mut self, represented_type: Option<&'static TypeInfo>) {
        if let Some(represented_type) = represented_type {
            assert!(
                matches!(represented_type, TypeInfo::Array(_)),
                "expected TypeInfo::Array but received: {represented_type:?}"
            );
        }

        self.represented_type = represented_type;
    }

    /// Takes the values that make up this dynamic array.
    pub fn into_values(self) -> Box<[DynamicValue]> {
        self.values
    }

    /// Gets the value of the element at the given index.
    ///
    /// Returns [`None`] if the index is invalid.
    #[inline]
    pub fn get(&self, index: usize) -> Option<&DynamicValue> {
        self.values.get(index)
    }

    /// Gets the value mutably of the element at the given index.
    ///
    /// Returns [`None`] if the index is invalid.
    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut DynamicValue> {
        self.values.get_mut(index)
    }

    /// Swaps two elements by their index.
    ///
    /// Panics if `a` or `b` are out of bounds.
    #[inline]
    pub fn swap(&mut self, a: usize, b: usize) {
        self.values.swap(a, b);
    }

    /// Returns the number of elements in the array.
    #[inline]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns whether the array is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Returns an iterator over the elements of this array.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &DynamicValue> + ExactSizeIterator {
        self.values.iter()
    }

    /// Gets the type info of the represented array, if this dynamic value represents a known type.
    pub fn get_represented_array_info(&self) -> Option<&'static ArrayInfo> {
        self.represented_type?.as_array().ok()
    }
}

impl Debug for DynamicArray {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "DynamicArray(")?;
        let mut debug = f.debug_list();
        for item in self.values.iter() {
            debug.entry(item as &dyn Debug);
        }
        debug.finish()?;
        write!(f, ")")
    }
}

impl<T: Into<DynamicValue>> FromIterator<T> for DynamicArray {
    fn from_iter<I: IntoIterator<Item = T>>(values: I) -> Self {
        Self {
            represented_type: None,
            values: values
                .into_iter()
                .map(Into::into)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        }
    }
}

impl IntoIterator for DynamicArray {
    type Item = DynamicValue;
    type IntoIter = alloc::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.into_vec().into_iter()
    }
}

impl_type_path!((in bevy_reflect::dynamic) DynamicArray);

/// An unordered mapping between reflected values.
#[derive(Default)]
pub struct DynamicMap {
    pub(crate) represented_type: Option<&'static TypeInfo>,
    hash_table: HashTable<(DynamicValue, DynamicValue)>,
}

impl DynamicMap {
    /// Creates an equivalent dynamic representation of `value`.
    ///
    /// This function clones opaque fields out of the original `value`, which can fail if a field
    /// does not implement [`Clone`].
    pub fn try_from_reflect(value: &dyn Map) -> Result<Self, ReflectCloneError> {
        let represented_type = value.get_represented_type_info();

        Ok(Self {
            represented_type,
            hash_table: Self::create_hash_table_from_iter(value.iter().map(|(key, value)| {
                Ok((
                    DynamicValue::try_from_reflect(key)?,
                    DynamicValue::try_from_reflect(value)?,
                ))
            }))?,
        })
    }

    /// Apply the values in `self` to the provided `reflect` using reflection.
    ///
    /// If this function fails, the `reflect` may be partially mutated.
    pub fn try_apply(mut self, reflect: &mut dyn Map) -> Result<(), ApplyError> {
        let mut result = Ok(());
        reflect.retain(&mut |k, v| {
            if result.is_err() {
                return true;
            }
            let k = k.try_as_reflect().unwrap();
            let v = v.try_as_reflect_mut().unwrap();
            let k_ref = DynamicValueRef::Concrete(k);
            let hash = DynamicMap::internal_hash(k_ref);
            let eq = DynamicMap::internal_eq(k_ref);
            match self.hash_table.find_entry(hash, eq) {
                Ok(entry) => {
                    let dynamic_value = entry.remove().0 .1;
                    result = dynamic_value.try_apply(v);
                    true
                }
                Err(_) => false,
            }
        });
        result?;

        for (key, value) in self.hash_table.into_iter() {
            let Some(key) = reflect.key_from_dynamic(key) else {
                return Err(ApplyError::InvalidDynamicValueForCollection);
            };
            let Some(value) = reflect.value_from_dynamic(value) else {
                return Err(ApplyError::InvalidDynamicValueForCollection);
            };
            reflect.insert_boxed(
                key.try_into_reflect().unwrap(),
                value.try_into_reflect().unwrap(),
            );
        }

        Ok(())
    }

    /// Attempts to clone this value's data.
    ///
    /// May fail if an opaque value cannot be cloned.
    pub fn dynamic_clone(&self) -> Result<Self, ReflectCloneError> {
        Ok(Self {
            represented_type: self.represented_type,
            hash_table: Self::create_hash_table_from_iter(
                self.hash_table
                    .iter()
                    .map(|(key, value)| Ok((key.dynamic_clone()?, value.dynamic_clone()?))),
            )?,
        })
    }

    /// Creates the internal hash table from an iterator of the dynamic values.
    fn create_hash_table_from_iter(
        iter: impl Iterator<Item = Result<(DynamicValue, DynamicValue), ReflectCloneError>>,
    ) -> Result<HashTable<(DynamicValue, DynamicValue)>, ReflectCloneError> {
        let mut hash_table = HashTable::new();
        for entry in iter {
            let (key, value) = entry?;
            let key_ref = key.as_ref();
            let hash = Self::internal_hash(key_ref);
            let eq = Self::internal_eq(key_ref);
            match hash_table.find_mut(hash, eq) {
                Some((_, old)) => {
                    *old = value;
                }
                None => {
                    hash_table.insert_unique(hash, (key, value), |(key, _)| {
                        Self::internal_hash(key.as_ref())
                    });
                }
            }
        }
        Ok(hash_table)
    }

    /// Sets the [type] to be represented by this `DynamicMap`.
    ///
    /// # Panics
    ///
    /// Panics if the given [type] is not a [`TypeInfo::Map`].
    ///
    /// [type]: TypeInfo
    pub fn set_represented_type(&mut self, represented_type: Option<&'static TypeInfo>) {
        if let Some(represented_type) = represented_type {
            assert!(
                matches!(represented_type, TypeInfo::Map(_)),
                "expected TypeInfo::Map but received: {represented_type:?}"
            );
        }

        self.represented_type = represented_type;
    }

    /// Inserts a typed key-value pair into the map.
    pub fn insert<K: Into<DynamicValue>, V: Into<DynamicValue>>(
        &mut self,
        key: K,
        value: V,
    ) -> Option<DynamicValue> {
        let key = key.into();
        let value = value.into();
        assert_eq!(
            key.as_partial_reflect()
                .reflect_partial_eq(key.as_partial_reflect()),
            Some(true),
            "keys inserted in `Map`-like types are expected to reflect `PartialEq`"
        );

        let key_ref = key.as_ref();
        let hash = Self::internal_hash(key_ref);
        let eq = Self::internal_eq(key_ref);
        match self.hash_table.find_mut(hash, eq) {
            Some((_, old)) => Some(core::mem::replace(old, value)),
            None => {
                self.hash_table
                    .insert_unique(hash, (key, value), |(key, _)| {
                        Self::internal_hash(key.as_ref())
                    });
                None
            }
        }
    }

    /// Computes a hash for the given `value`.
    ///
    /// Used to maintain [`Self::hash_table`].
    fn internal_hash(value: DynamicValueRef<'_>) -> u64 {
        match value.dynamic_hash() {
            Some(hash) => hash,
            None => hash_error(value),
        }
    }

    /// Returns a function to check equality of `key` with a given element.
    ///
    /// Used to maintain [`Self::hash_table`].
    fn internal_eq(
        key: DynamicValueRef<'_>,
    ) -> impl FnMut(&(DynamicValue, DynamicValue)) -> bool + '_ {
        move |(other, _)| {
            key
            .dynamic_partial_eq(other.as_ref())
            .expect("underlying type does not reflect `PartialEq` and hence doesn't support equality checks")
        }
    }

    /// Gets the value corresponding to the given `key` in this map.
    ///
    /// Returns [`None`] if a corresponding value could not be found.
    pub fn get(&self, key: DynamicValueRef<'_>) -> Option<&DynamicValue> {
        self.hash_table
            .find(Self::internal_hash(key), Self::internal_eq(key))
            .map(|(_, value)| value)
    }

    /// Gets the value mutably corresponding to the given `key` in this map.
    ///
    /// Returns [`None`] if a corresponding value could not be found.
    pub fn get_mut(&mut self, key: DynamicValueRef<'_>) -> Option<&mut DynamicValue> {
        self.hash_table
            .find_mut(Self::internal_hash(key), Self::internal_eq(key))
            .map(|(_, value)| value)
    }

    /// Returns the number of entries in the map.
    pub fn len(&self) -> usize {
        self.hash_table.len()
    }

    /// Returns whether the map is empty.
    pub fn is_empty(&self) -> bool {
        self.hash_table.is_empty()
    }

    /// Returns an iterator over the entries of this map.
    pub fn iter(&self) -> impl Iterator<Item = (&DynamicValue, &DynamicValue)> + ExactSizeIterator {
        self.hash_table.iter().map(|(k, v)| (k, v))
    }

    /// Removes all entries from this map, returning the entries in a [`Vec`].
    pub fn drain(&mut self) -> Vec<(Box<dyn PartialReflect>, Box<dyn PartialReflect>)> {
        self.hash_table
            .drain()
            .map(|(key, value)| {
                // Note: the non-dynamic maps also Box their keys/values, so it's ok to do that here
                // too.
                (
                    key.as_boxed_partial_reflect(),
                    value.as_boxed_partial_reflect(),
                )
            })
            .collect()
    }

    /// Removes any entries that return `false` when evaluated by `f`.
    pub fn retain(&mut self, f: &mut dyn FnMut(&DynamicValue, &mut DynamicValue) -> bool) {
        self.hash_table.retain(move |(key, value)| f(key, value));
    }

    /// Removes the entry whose key is equivalent to `key`.
    pub fn remove(&mut self, key: DynamicValueRef<'_>) -> Option<DynamicValue> {
        let hash = Self::internal_hash(key);
        let eq = Self::internal_eq(key);
        match self.hash_table.find_entry(hash, eq) {
            Ok(entry) => {
                let ((_, old_value), _) = entry.remove();
                Some(old_value)
            }
            Err(_) => None,
        }
    }

    /// Gets the type info of the represented map, if this dynamic value represents a known type.
    pub fn get_represented_map_info(&self) -> Option<&'static MapInfo> {
        self.represented_type?.as_map().ok()
    }
}

impl Debug for DynamicMap {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "DynamicMap(")?;
        let mut debug = f.debug_map();
        for (key, value) in self.iter() {
            debug.entry(key as &dyn Debug, value as &dyn Debug);
        }
        debug.finish()?;
        write!(f, ")")
    }
}

impl_type_path!((in bevy_reflect::dynamic) DynamicMap);

impl<K: Into<DynamicValue>, V: Into<DynamicValue>> FromIterator<(K, V)> for DynamicMap {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(items: I) -> Self {
        let mut map = Self::default();
        for (key, value) in items.into_iter() {
            map.insert(key, value);
        }
        map
    }
}

impl IntoIterator for DynamicMap {
    type Item = (DynamicValue, DynamicValue);
    type IntoIter = bevy_platform::collections::hash_table::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.hash_table.into_iter()
    }
}

/// An unordered set of reflected values.
#[derive(Default)]
pub struct DynamicSet {
    pub(crate) represented_type: Option<&'static TypeInfo>,
    hash_table: HashTable<DynamicValue>,
}

impl DynamicSet {
    /// Creates an equivalent dynamic representation of `value`.
    ///
    /// This function clones opaque fields out of the original `value`, which can fail if a field
    /// does not implement [`Clone`].
    pub fn try_from_reflect(value: &dyn Set) -> Result<Self, ReflectCloneError> {
        let represented_type = value.get_represented_type_info();

        Ok(Self {
            represented_type,
            hash_table: Self::create_hash_table_from_iter(
                value.iter().map(DynamicValue::try_from_reflect),
            )?,
        })
    }

    /// Apply the values in `self` to the provided `reflect` using reflection.
    ///
    /// If this function fails, the `reflect` may be partially mutated.
    pub fn try_apply(mut self, reflect: &mut dyn Set) -> Result<(), ApplyError> {
        reflect.retain(&mut |reflect_value| {
            let reflect_value = reflect_value.try_as_reflect().unwrap();
            let reflect_value_ref = DynamicValueRef::Concrete(reflect_value);
            let hash = DynamicSet::internal_hash(reflect_value_ref);
            let eq = DynamicSet::internal_eq(reflect_value_ref);
            match self.hash_table.find_entry(hash, eq) {
                Ok(entry) => {
                    entry.remove();
                    true
                }
                Err(_) => false,
            }
        });

        for value in self.hash_table.into_iter() {
            let Some(value) = reflect.value_from_dynamic(value) else {
                return Err(ApplyError::InvalidDynamicValueForCollection);
            };
            reflect.insert_boxed(value.try_into_reflect().unwrap());
        }
        Ok(())
    }

    /// Attempts to clone this value's data.
    ///
    /// May fail if an opaque value cannot be cloned.
    pub fn dynamic_clone(&self) -> Result<Self, ReflectCloneError> {
        Ok(Self {
            represented_type: self.represented_type,
            hash_table: Self::create_hash_table_from_iter(
                self.hash_table.iter().map(DynamicValue::dynamic_clone),
            )?,
        })
    }

    /// Creates the internal hash table from an iterator of the dynamic values.
    fn create_hash_table_from_iter(
        iter: impl Iterator<Item = Result<DynamicValue, ReflectCloneError>>,
    ) -> Result<HashTable<DynamicValue>, ReflectCloneError> {
        let mut hash_table = HashTable::new();
        for entry in iter {
            let value = entry?;
            let value_ref = value.as_ref();
            let hash = Self::internal_hash(value_ref);
            let eq = Self::internal_eq(value_ref);
            match hash_table.find_mut(hash, eq) {
                Some(old) => {
                    *old = value;
                }
                None => {
                    hash_table
                        .insert_unique(hash, value, |value| Self::internal_hash(value.as_ref()));
                }
            }
        }
        Ok(hash_table)
    }

    /// Sets the [type] to be represented by this `DynamicSet`.
    ///
    /// # Panics
    ///
    /// Panics if the given [type] is not a [`TypeInfo::Set`].
    ///
    /// [type]: TypeInfo
    pub fn set_represented_type(&mut self, represented_type: Option<&'static TypeInfo>) {
        if let Some(represented_type) = represented_type {
            assert!(
                matches!(represented_type, TypeInfo::Set(_)),
                "expected TypeInfo::Set but received: {represented_type:?}"
            );
        }

        self.represented_type = represented_type;
    }

    /// Inserts a typed value into the set.
    pub fn insert(&mut self, value: impl Into<DynamicValue>) -> bool {
        let value = value.into();
        assert_eq!(
            value
                .as_partial_reflect()
                .reflect_partial_eq(value.as_partial_reflect()),
            Some(true),
            "Values inserted in `Set` like types are expected to reflect `PartialEq`"
        );
        let value_ref = value.as_ref();
        let hash = Self::internal_hash(value_ref);
        match self.hash_table.find_mut(hash, Self::internal_eq(value_ref)) {
            Some(old) => {
                *old = value;
                false
            }
            None => {
                self.hash_table
                    .insert_unique(hash, value, |value| Self::internal_hash(value.as_ref()));
                true
            }
        }
    }

    /// Computes a hash for the given `value`.
    ///
    /// Used to maintain [`Self::hash_table`].
    fn internal_hash(value: DynamicValueRef<'_>) -> u64 {
        match value.dynamic_hash() {
            Some(hash) => hash,
            None => hash_error(value),
        }
    }

    /// Returns a function to check equality of `value` with a given element.
    ///
    /// Used to maintain [`Self::hash_table`].
    fn internal_eq(value: DynamicValueRef<'_>) -> impl FnMut(&DynamicValue) -> bool + '_ {
        move |other| {
            value
                .dynamic_partial_eq(other.as_ref())
                .expect("Underlying type does not reflect `PartialEq` and hence doesn't support equality checks")
        }
    }

    /// Gets the value equivalent to `value`.
    ///
    /// Returns [`None`] if an equivalent value could not be found.
    pub fn get(&self, value: DynamicValueRef<'_>) -> Option<&DynamicValue> {
        self.hash_table
            .find(Self::internal_hash(value), Self::internal_eq(value))
    }

    /// Returns the number of entries in the set.
    pub fn len(&self) -> usize {
        self.hash_table.len()
    }

    /// Returns whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.hash_table.is_empty()
    }

    /// Returns an iterator over the values of this set.
    pub fn iter(&self) -> impl Iterator<Item = &DynamicValue> + ExactSizeIterator {
        self.hash_table.iter()
    }

    /// Removes all values in this set, returning the values as a [`Vec`].
    pub fn drain(&mut self) -> Vec<DynamicValue> {
        self.hash_table.drain().collect()
    }

    /// Removes any values that return `false` when evaluated by `f`.
    pub fn retain(&mut self, f: &mut dyn FnMut(&DynamicValue) -> bool) {
        self.hash_table.retain(move |value| f(value));
    }

    /// Removes the value that is equivalent to `value`.
    pub fn remove(&mut self, value: DynamicValueRef<'_>) -> bool {
        self.hash_table
            .find_entry(Self::internal_hash(value), Self::internal_eq(value))
            .map(HashTableOccupiedEntry::remove)
            .is_ok()
    }

    /// Returns whether a value equivalent to `value` is present in this set.
    pub fn contains(&self, value: DynamicValueRef<'_>) -> bool {
        self.hash_table
            .find(Self::internal_hash(value), Self::internal_eq(value))
            .is_some()
    }

    /// Gets the type info of the represented set, if this dynamic value represents a known type.
    pub fn get_represented_set_info(&self) -> Option<&'static SetInfo> {
        self.represented_type?.as_set().ok()
    }
}

impl Debug for DynamicSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "DynamicSet(")?;
        let mut debug = f.debug_set();
        for value in self.iter() {
            debug.entry(value as &dyn Debug);
        }
        debug.finish()?;
        write!(f, ")")
    }
}

impl_type_path!((in bevy_reflect::dynamic) DynamicSet);

impl<T: Into<DynamicValue>> FromIterator<T> for DynamicSet {
    fn from_iter<I: IntoIterator<Item = T>>(values: I) -> Self {
        let mut this = Self {
            represented_type: None,
            hash_table: HashTable::new(),
        };

        for value in values {
            this.insert(value);
        }

        this
    }
}

impl IntoIterator for DynamicSet {
    type Item = DynamicValue;
    type IntoIter = bevy_platform::collections::hash_table::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.hash_table.into_iter()
    }
}

/// Panics with a detailed error message when hashing a dynamic value fails.
fn hash_error(value: DynamicValueRef<'_>) -> ! {
    match value {
        DynamicValueRef::Concrete(value) => {
            panic!(
                "failed to hash: the given key of type `{}` does not support hashing",
                value.reflect_type_info().type_path()
            )
        }
        _ => match value.get_represented_type_info() {
            None => panic!("the DynamicValue does not support hashing"),
            Some(type_info) => panic!(
                "the DynamicValue (representing `{}`) does not support hashing",
                type_info.type_path()
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::borrow::ToOwned;
    use bevy_platform::collections::HashSet;

    #[derive(Reflect, Default)]
    struct MyStruct {
        a: (),
        b: (),
        c: (),
    }

    #[derive(Reflect, Default)]
    struct OtherStruct {
        a: u32,
        b: u64,
        c: MyStruct,
    }

    #[test]
    fn dynamic_struct_remove_at() {
        let mut s = DynamicStruct::try_from_reflect(&OtherStruct::default()).unwrap();

        assert_eq!(s.field_len(), 3);

        let field_2 = s
            .remove_at(1)
            .expect("Invalid index for `s.remove_at(index)`");

        assert_eq!(s.field_len(), 2);
        assert_eq!(field_2.0, "b");

        let field_3 = s
            .remove_at(0)
            .expect("Invalid index for `s.remove_at(index)`");

        assert_eq!(s.field_len(), 1);
        assert_eq!(field_3.0, "a");

        let field_1 = s
            .remove_at(0)
            .expect("Invalid index for `s.remove_at(index)`");

        assert_eq!(s.field_len(), 0);
        assert_eq!(field_1.0, "c");
    }

    #[test]
    fn dynamic_struct_remove_by_name() {
        let mut s = DynamicStruct::try_from_reflect(&OtherStruct::default()).unwrap();

        assert_eq!(s.field_len(), 3);

        let field_3 = s
            .remove_by_name("b")
            .expect("Invalid name for `s.remove_by_name(name)`");

        assert_eq!(s.field_len(), 2);
        assert_eq!(field_3.0, "b");

        let field_2 = s
            .remove_by_name("c")
            .expect("Invalid name for `s.remove_by_name(name)`");

        assert_eq!(s.field_len(), 1);
        assert_eq!(field_2.0, "c");

        let field_1 = s
            .remove_by_name("a")
            .expect("Invalid name for `s.remove_by_name(name)`");

        assert_eq!(s.field_len(), 0);
        assert_eq!(field_1.0, "a");
    }

    #[test]
    fn dynamic_struct_remove_if() {
        let mut s = DynamicStruct::try_from_reflect(&OtherStruct::default()).unwrap();

        assert_eq!(s.field_len(), 3);

        let field_3_name = s
            .name_at(2)
            .expect("Invalid index for `s.field_at(index)`")
            .to_owned();
        let field_3 = s
            .remove_if(|(name, _field)| name == field_3_name)
            .expect("No valid name/field found for `s.remove_with(|(name, field)|{})");

        assert_eq!(s.field_len(), 2);
        assert_eq!(field_3.0, "c");
    }

    #[test]
    fn dynamic_struct_remove_combo() {
        let mut s = DynamicStruct::try_from_reflect(&OtherStruct::default()).unwrap();

        assert_eq!(s.field_len(), 3);

        let field_2 = s
            .remove_at(
                s.index_of_name("b")
                    .expect("Invalid name for `s.index_of_name(name)`"),
            )
            .expect("Invalid index for `s.remove_at(index)`");

        assert_eq!(s.field_len(), 2);
        assert_eq!(field_2.0, "b");

        let field_3_name = s
            .name_at(1)
            .expect("Invalid name for s.name_at(index)")
            .to_owned();

        let field_3 = s
            .remove_by_name(field_3_name.as_ref())
            .expect("Invalid name for `s.remove_by_name(name)`");

        assert_eq!(s.field_len(), 1);
        assert_eq!(field_3.0, "c");

        let field_1_name = s
            .name_at(0)
            .expect("Invalid name for `s.name_at(name)`")
            .to_owned();
        let field_1 = s
            .remove_if(|(name, _field)| name == field_1_name)
            .expect("No valid name/field found for `s.remove_with(|(name, field)|{})`");

        assert_eq!(s.field_len(), 0);
        assert_eq!(field_1.0, "a");
    }

    #[test]
    fn list_into_iter() {
        let mut list = DynamicList::default();
        list.push(0usize);
        list.push(1usize);
        list.push(2usize);
        let items = list.into_iter();
        for (index, item) in items.into_iter().enumerate() {
            let value = item
                .try_take::<usize>()
                .expect("couldn't downcast to usize");
            assert_eq!(index, value);
        }
    }

    #[test]
    fn list_swap() {
        let mut list = DynamicList::default();
        list.push(0usize);
        list.push(1usize);
        list.swap(0, 1);
        assert_eq!(
            list.get(0)
                .unwrap()
                .as_concrete()
                .unwrap()
                .downcast_ref::<usize>(),
            Some(&1usize)
        );
        assert_eq!(
            list.get(1)
                .unwrap()
                .as_concrete()
                .unwrap()
                .downcast_ref::<usize>(),
            Some(&0usize)
        );
    }

    #[test]
    #[should_panic]
    fn list_swap_oob() {
        let mut list = DynamicList::default();
        list.push(0usize);
        list.push(1usize);
        list.swap(0, 10);
    }

    #[test]
    fn map_remove() {
        let mut map = DynamicMap::default();
        map.insert(0, 0);
        map.insert(1, 1);

        assert_eq!(
            map.remove((&0).into())
                .unwrap()
                .as_concrete()
                .unwrap()
                .downcast_ref(),
            Some(&0)
        );
        assert!(map.get((&0).into()).is_none());
        assert_eq!(
            map.get((&1).into())
                .unwrap()
                .as_concrete()
                .unwrap()
                .downcast_ref(),
            Some(&1)
        );

        assert_eq!(
            map.remove((&1).into())
                .unwrap()
                .as_concrete()
                .unwrap()
                .downcast_ref(),
            Some(&1)
        );
        assert!(map.get((&1).into()).is_none());

        assert!(map.remove((&1).into()).is_none());
        assert!(map.get((&1).into()).is_none());
    }

    #[test]
    fn map_apply() {
        let mut map_a = HashMap::new();
        map_a.insert(0, 0);
        map_a.insert(1, 1);

        let mut map_b = DynamicMap::default();
        map_b.insert(10, 10);
        map_b.insert(1, 5);

        map_b.try_apply(&mut map_a).unwrap();

        assert!(map_a.get(&0).is_none());
        assert_eq!(map_a.get(&1), Some(&5));
        assert_eq!(map_a.get(&10), Some(&10));
    }

    #[test]
    fn set_into_iter() {
        let expected = ["foo", "bar", "baz"];

        let mut set = DynamicSet::default();
        set.insert(expected[0].to_string());
        set.insert(expected[1].to_string());
        set.insert(expected[2].to_string());

        for item in set.into_iter() {
            let value = item
                .try_take::<String>()
                .expect("couldn't downcast to String");
            let index = expected
                .as_slice()
                .iter()
                .position(|i| *i == value.as_str())
                .expect("Element found in expected array");
            assert_eq!(expected[index], value);
        }
    }

    #[test]
    fn set_apply() {
        let mut map_a = HashSet::new();
        map_a.insert(0);
        map_a.insert(1);

        let mut map_b = DynamicSet::default();
        map_b.insert(1);
        map_b.insert(2);

        map_b.try_apply(&mut map_a).unwrap();

        assert!(map_a.get(&0).is_none());
        assert_eq!(map_a.get(&1), Some(&1));
        assert_eq!(map_a.get(&2), Some(&2));
    }
}

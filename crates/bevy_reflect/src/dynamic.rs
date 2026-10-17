//! Contains types for holding dynamic representations of other types.

use core::{any::Any, mem::transmute};

use crate::{
    array::{Array, DynamicArray},
    enums::{DynamicEnum, Enum},
    list::{DynamicList, List},
    map::{DynamicMap, Map},
    set::{DynamicSet, Set},
    structs::{DynamicStruct, Struct},
    tuple::{DynamicTuple, Tuple},
    tuple_struct::{DynamicTupleStruct, TupleStruct},
    ApplyError, PartialReflect, Reflect, ReflectCloneError, ReflectRef, TypeInfo, TypePath,
};

use alloc::{boxed::Box, string::String, vec::Vec};
use bevy_reflect_derive::impl_type_path;

/// A dynamically typed value based on reflection.
///
/// A value can either be a "concrete" [`Reflect`] value, or it may be a dynamic representation such
/// as [`DynamicStruct`].
#[derive(Debug)]
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
        match self {
            Self::Concrete(inner_box) => {
                if let Ok(value) = inner_box.reflect_clone() {
                    return Ok(Self::Concrete(value));
                }

                Self::try_from_reflect(&**inner_box)
            }
            Self::Struct(value) => Ok(Self::Struct(DynamicStruct::dynamic_clone(value)?)),
            Self::TupleStruct(value) => {
                Ok(Self::TupleStruct(DynamicTupleStruct::dynamic_clone(value)?))
            }
            Self::Tuple(value) => Ok(Self::Tuple(DynamicTuple::dynamic_clone(value)?)),
            Self::List(value) => Ok(Self::List(DynamicList::dynamic_clone(value)?)),
            Self::Array(value) => Ok(Self::Array(DynamicArray::dynamic_clone(value)?)),
            Self::Map(value) => Ok(Self::Map(DynamicMap::dynamic_clone(value)?)),
            Self::Set(value) => Ok(Self::Set(DynamicSet::dynamic_clone(value)?)),
            Self::Enum(value) => Ok(Self::Enum(DynamicEnum::dynamic_clone(value)?)),
        }
    }

    /// Gets the type that this dynamic representation represents.
    pub fn get_represented_type_into(&self) -> Option<&'static TypeInfo> {
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
}

macro_rules! forward_method {
    ($self:expr, $value:ident, $ex:expr) => {
        match $self {
            DynamicValue::Struct($value) => $ex,
            DynamicValue::TupleStruct($value) => $ex,
            DynamicValue::Tuple($value) => $ex,
            DynamicValue::List($value) => $ex,
            DynamicValue::Array($value) => $ex,
            DynamicValue::Map($value) => $ex,
            DynamicValue::Set($value) => $ex,
            DynamicValue::Enum($value) => $ex,
            DynamicValue::Concrete($value) => $ex,
        }
    };
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

impl_type_path!((in bevy_reflect) DynamicValueWrapper);

impl PartialReflect for DynamicValueWrapper {
    fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        forward_method!(&self.0, value, value.get_represented_type_info())
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
        forward_method!(&mut self.0, self_value, self_value.try_apply(value))
    }

    fn reflect_kind(&self) -> crate::ReflectKind {
        forward_method!(&self.0, value, value.reflect_kind())
    }

    fn reflect_ref(&self) -> ReflectRef<'_> {
        forward_method!(&self.0, value, value.reflect_ref())
    }

    fn reflect_mut(&mut self) -> crate::ReflectMut<'_> {
        forward_method!(&mut self.0, value, value.reflect_mut())
    }

    fn reflect_owned(self: Box<Self>) -> crate::ReflectOwned {
        match self.0 {
            DynamicValue::Struct(_) => crate::ReflectOwned::Struct(self),
            DynamicValue::TupleStruct(_) => crate::ReflectOwned::TupleStruct(self),
            DynamicValue::Tuple(_) => crate::ReflectOwned::Tuple(self),
            DynamicValue::List(_) => crate::ReflectOwned::List(self),
            DynamicValue::Array(_) => crate::ReflectOwned::Array(self),
            DynamicValue::Map(_) => crate::ReflectOwned::Map(self),
            DynamicValue::Set(_) => crate::ReflectOwned::Set(self),
            DynamicValue::Enum(_) => crate::ReflectOwned::Enum(self),
            // We always bail early if `DynamicValue` stores Concrete, so we never create a wrapper
            // with this variant.
            DynamicValue::Concrete(_) => unreachable!(),
        }
    }

    fn reflect_hash(&self) -> Option<u64> {
        forward_method!(&self.0, value, value.reflect_hash())
    }

    fn reflect_partial_eq(&self, value: &dyn PartialReflect) -> Option<bool> {
        forward_method!(&self.0, self_value, self_value.reflect_partial_eq(value))
    }

    fn reflect_partial_cmp(&self, value: &dyn PartialReflect) -> Option<core::cmp::Ordering> {
        forward_method!(&self.0, self_value, self_value.reflect_partial_cmp(value))
    }

    fn debug(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        forward_method!(&self.0, value, value.debug(f))
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
        self.as_struct().field(name)
    }

    fn field_mut(&mut self, name: &str) -> Option<&mut dyn PartialReflect> {
        self.as_struct_mut().field_mut(name)
    }

    fn field_at(&self, index: usize) -> Option<&dyn PartialReflect> {
        self.as_struct().field_at(index)
    }

    fn field_at_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect> {
        self.as_struct_mut().field_at_mut(index)
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

    fn iter_fields(&self) -> crate::structs::FieldIter<'_> {
        self.as_struct().iter_fields()
    }

    fn get_represented_struct_info(&self) -> Option<&'static crate::structs::StructInfo> {
        self.as_struct().get_represented_struct_info()
    }
}

impl TupleStruct for DynamicValueWrapper {
    fn field(&self, index: usize) -> Option<&dyn PartialReflect> {
        self.as_tuple_struct().field(index)
    }

    fn field_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect> {
        self.as_tuple_struct_mut().field_mut(index)
    }

    fn field_len(&self) -> usize {
        self.as_tuple_struct().field_len()
    }

    fn iter_fields(&self) -> crate::tuple_struct::TupleStructFieldIter<'_> {
        self.as_tuple_struct().iter_fields()
    }
}

impl Tuple for DynamicValueWrapper {
    fn field(&self, index: usize) -> Option<&dyn PartialReflect> {
        self.as_tuple().field(index)
    }

    fn field_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect> {
        self.as_tuple_mut().field_mut(index)
    }

    fn field_len(&self) -> usize {
        self.as_tuple().field_len()
    }

    fn iter_fields(&self) -> crate::tuple::TupleFieldIter<'_> {
        self.as_tuple().iter_fields()
    }

    fn drain(self: Box<Self>) -> Vec<Box<dyn PartialReflect>> {
        self.as_tuple_owned().into_boxed_fields()
    }

    fn get_represented_tuple_info(&self) -> Option<&'static crate::tuple::TupleInfo> {
        self.as_tuple().get_represented_tuple_info()
    }
}

impl List for DynamicValueWrapper {
    fn get(&self, index: usize) -> Option<&dyn PartialReflect> {
        self.as_list().get(index)
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect> {
        self.as_list_mut().get_mut(index)
    }

    fn insert(&mut self, index: usize, element: Box<dyn PartialReflect>) {
        self.as_list_mut().insert(index, element);
    }

    fn remove(&mut self, index: usize) -> Box<dyn PartialReflect> {
        self.as_list_mut().remove(index)
    }

    fn swap(&mut self, a: usize, b: usize) {
        self.as_list_mut().swap(a, b);
    }

    fn len(&self) -> usize {
        self.as_list().len()
    }

    fn iter(&self) -> crate::list::ListIter<'_> {
        self.as_list().iter()
    }

    fn drain(&mut self) -> Vec<Box<dyn PartialReflect>> {
        self.as_list_mut().drain()
    }

    fn push(&mut self, value: Box<dyn PartialReflect>) {
        (self.as_list_mut() as &mut dyn List).push(value);
    }

    fn pop(&mut self) -> Option<Box<dyn PartialReflect>> {
        self.as_list_mut().pop()
    }

    fn is_empty(&self) -> bool {
        self.as_list().is_empty()
    }

    fn get_represented_list_info(&self) -> Option<&'static crate::list::ListInfo> {
        self.as_list().get_represented_list_info()
    }
}

impl Array for DynamicValueWrapper {
    fn get(&self, index: usize) -> Option<&dyn PartialReflect> {
        self.as_array().get(index)
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect> {
        self.as_array_mut().get_mut(index)
    }

    fn swap(&mut self, a: usize, b: usize) {
        self.as_array_mut().swap(a, b);
    }

    fn len(&self) -> usize {
        self.as_array().len()
    }

    fn iter(&self) -> crate::array::ArrayIter<'_> {
        self.as_array().iter()
    }

    fn drain(self: Box<Self>) -> Vec<Box<dyn PartialReflect>> {
        self.as_array_owned().into_boxed_values()
    }

    fn is_empty(&self) -> bool {
        self.as_array().is_empty()
    }

    fn get_represented_array_info(&self) -> Option<&'static crate::array::ArrayInfo> {
        self.as_array().get_represented_array_info()
    }
}

impl Map for DynamicValueWrapper {
    fn get(&self, key: &dyn PartialReflect) -> Option<&dyn PartialReflect> {
        self.as_map().get(key)
    }

    fn get_mut(&mut self, key: &dyn PartialReflect) -> Option<&mut dyn PartialReflect> {
        self.as_map_mut().get_mut(key)
    }

    fn len(&self) -> usize {
        self.as_map().len()
    }

    fn iter(&self) -> Box<dyn Iterator<Item = (&dyn PartialReflect, &dyn PartialReflect)> + '_> {
        self.as_map().iter()
    }

    fn drain(&mut self) -> Vec<(Box<dyn PartialReflect>, Box<dyn PartialReflect>)> {
        self.as_map_mut().drain()
    }

    fn retain(&mut self, f: &mut dyn FnMut(&dyn PartialReflect, &mut dyn PartialReflect) -> bool) {
        self.as_map_mut().retain(f);
    }

    fn insert_boxed(
        &mut self,
        key: Box<dyn PartialReflect>,
        value: Box<dyn PartialReflect>,
    ) -> Option<Box<dyn PartialReflect>> {
        self.as_map_mut().insert_boxed(key, value)
    }

    fn remove(&mut self, key: &dyn PartialReflect) -> Option<Box<dyn PartialReflect>> {
        self.as_map_mut().remove(key)
    }

    fn is_empty(&self) -> bool {
        self.as_map().is_empty()
    }

    fn get_represented_map_info(&self) -> Option<&'static crate::map::MapInfo> {
        self.as_map().get_represented_map_info()
    }
}

impl Set for DynamicValueWrapper {
    fn get(&self, value: &dyn PartialReflect) -> Option<&dyn PartialReflect> {
        self.as_set().get(value)
    }

    fn len(&self) -> usize {
        self.as_set().len()
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &dyn PartialReflect> + '_> {
        self.as_set().iter()
    }

    fn drain(&mut self) -> Vec<Box<dyn PartialReflect>> {
        self.as_set_mut().drain()
    }

    fn retain(&mut self, f: &mut dyn FnMut(&dyn PartialReflect) -> bool) {
        self.as_set_mut().retain(f);
    }

    fn insert_boxed(&mut self, value: Box<dyn PartialReflect>) -> bool {
        self.as_set_mut().insert_boxed(value)
    }

    fn remove(&mut self, value: &dyn PartialReflect) -> bool {
        self.as_set_mut().remove(value)
    }

    fn contains(&self, value: &dyn PartialReflect) -> bool {
        self.as_set().contains(value)
    }

    fn is_empty(&self) -> bool {
        self.as_set().is_empty()
    }
}

impl Enum for DynamicValueWrapper {
    fn field(&self, name: &str) -> Option<&dyn PartialReflect> {
        self.as_enum().field(name)
    }

    fn field_at(&self, index: usize) -> Option<&dyn PartialReflect> {
        self.as_enum().field_at(index)
    }

    fn field_mut(&mut self, name: &str) -> Option<&mut dyn PartialReflect> {
        self.as_enum_mut().field_mut(name)
    }

    fn field_at_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect> {
        self.as_enum_mut().field_at_mut(index)
    }

    fn index_of(&self, name: &str) -> Option<usize> {
        self.as_enum().index_of(name)
    }

    fn name_at(&self, index: usize) -> Option<&str> {
        self.as_enum().name_at(index)
    }

    fn iter_fields(&self) -> crate::enums::VariantFieldIter<'_> {
        self.as_enum().iter_fields()
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
        self.as_enum().is_variant(variant_type)
    }

    fn variant_path(&self) -> String {
        self.as_enum().variant_path()
    }

    fn get_represented_enum_info(&self) -> Option<&'static crate::enums::EnumInfo> {
        self.as_enum().get_represented_enum_info()
    }
}

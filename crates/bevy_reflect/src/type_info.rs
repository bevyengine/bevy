use std::any::TypeId;
use crate::{ListInfo, MapInfo, Reflect, StructInfo, TupleInfo, TupleStructInfo};
use std::borrow::{Borrow, Cow};

/// Compile-time type information for various object types
#[derive(Debug, Clone)]
pub enum TypeInfo {
    Struct(StructInfo),
    TupleStruct(TupleStructInfo),
    Tuple(TupleInfo),
    List(ListInfo),
    Map(MapInfo),
    Value(ValueInfo),
    /// Type information for "dynamic" types whose metadata can't be known at compile-time
    ///
    /// This includes structs like [`DynamicStruct`](crate::DynamicStruct) and [`DynamicList`](crate::DynamicList)
    Dynamic(DynamicInfo),
}

/// A container for compile-time info related to general value types, including primitives
#[derive(Debug, Clone)]
pub struct ValueInfo {
    type_name: Cow<'static, str>,
    type_id: TypeId
}

impl ValueInfo {
    pub fn new<T: Reflect>() -> Self {
        Self {
            type_name: Cow::Owned(std::any::type_name::<T>().to_string()),
            type_id: TypeId::of::<T>()
        }
    }

    /// The type name of this value
    pub fn type_name(&self) -> &str {
        self.type_name.borrow()
    }

    /// The `TypeId` of this value
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Check if the given type matches this value's type
    pub fn is<T: Reflect>(&self) -> bool {
        TypeId::of::<T>() == self.type_id
    }
}

#[derive(Debug, Clone)]
pub struct DynamicInfo {
    type_name: Cow<'static, str>,
    type_id: TypeId
}

impl DynamicInfo {
    pub fn new<T: Reflect>() -> Self {
        Self {
            type_name: Cow::Owned(std::any::type_name::<T>().to_string()),
            type_id: TypeId::of::<T>()
        }
    }

    /// The type name of this value
    pub fn type_name(&self) -> &str {
        self.type_name.borrow()
    }

    /// The `TypeId` of this value
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Check if the given type matches this value's type
    pub fn is<T: Reflect>(&self) -> bool {
        TypeId::of::<T>() == self.type_id
    }
}
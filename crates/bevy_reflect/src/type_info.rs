use crate::{ListInfo, MapInfo, Reflect, StructInfo, TupleInfo, TupleStructInfo};
use std::any::TypeId;
use std::borrow::{Borrow, Cow};

/// A static accessor to compile-time type information
///
/// This is used by the `#[derive(Reflect)]` macro to generate an implementation
/// of [`TypeInfo`] to pass to register via [`TypeRegistration::of`][0].
///
/// [0]: crate::TypeRegistration::of
pub trait Typed: Reflect {
    /// Returns the compile-time info for the underlying type
    fn type_info() -> TypeInfo;
}

/// Compile-time type information for various reflected types
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

impl TypeInfo {
    /// The name of the reflected type
    pub fn type_name(&self) -> &str {
        match self {
            Self::Struct(info) => info.type_name(),
            Self::TupleStruct(info) => info.type_name(),
            Self::Tuple(info) => info.type_name(),
            Self::List(info) => info.type_name(),
            Self::Map(info) => info.type_name(),
            Self::Value(info) => info.type_name(),
            Self::Dynamic(info) => info.type_name(),
        }
    }

    /// The `TypeId` of the reflected type
    pub fn type_id(&self) -> TypeId {
        match self {
            Self::Struct(info) => info.type_id(),
            Self::TupleStruct(info) => info.type_id(),
            Self::Tuple(info) => info.type_id(),
            Self::List(info) => info.type_id(),
            Self::Map(info) => info.type_id(),
            Self::Value(info) => info.type_id(),
            Self::Dynamic(info) => info.type_id(),
        }
    }
}

/// A container for compile-time info related to general value types, including primitives
#[derive(Debug, Clone)]
pub struct ValueInfo {
    type_name: Cow<'static, str>,
    type_id: TypeId,
}

impl ValueInfo {
    pub fn new<T: Reflect>() -> Self {
        Self {
            type_name: Cow::Owned(std::any::type_name::<T>().to_string()),
            type_id: TypeId::of::<T>(),
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
    type_id: TypeId,
}

impl DynamicInfo {
    pub fn new<T: Reflect>() -> Self {
        Self {
            type_name: Cow::Owned(std::any::type_name::<T>().to_string()),
            type_id: TypeId::of::<T>(),
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

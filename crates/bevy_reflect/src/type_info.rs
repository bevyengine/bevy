use crate::{ListInfo, MapInfo, Reflect, StructInfo, TupleInfo, TupleStructInfo};
use std::any::{Any, TypeId};

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

#[derive(Clone, Debug)]
/// Type information used to identify a given type, including the type name and its [`TypeId`].
pub struct TypeIdentity(&'static str, TypeId);

impl TypeIdentity {
    /// Creates a new [`TypeIdentity`] with the given type name and [`TypeId`]
    pub const fn new(name: &'static str, type_id: TypeId) -> Self {
        Self(name, type_id)
    }

    /// Creates a new [`TypeIdentity`] for the given type
    pub fn of<T: Any + ?Sized>() -> Self {
        Self(std::any::type_name::<T>(), TypeId::of::<T>())
    }

    /// The name of this type
    pub fn type_name(&self) -> &'static str {
        self.0
    }

    /// The [`TypeId`] of this type
    pub fn type_id(&self) -> TypeId {
        self.1
    }

    /// Check if the given type matches this type
    pub fn is<T: Any>(&self) -> bool {
        TypeId::of::<T>() == self.1
    }
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
    /// The [`TypeIdentity`] of the reflected type
    pub fn id(&self) -> &TypeIdentity {
        match self {
            Self::Struct(info) => info.id(),
            Self::TupleStruct(info) => info.id(),
            Self::Tuple(info) => info.id(),
            Self::List(info) => info.id(),
            Self::Map(info) => info.id(),
            Self::Value(info) => info.id(),
            Self::Dynamic(info) => info.id(),
        }
    }
}

/// A container for compile-time info related to general value types, including primitives
#[derive(Debug, Clone)]
pub struct ValueInfo {
    id: TypeIdentity,
}

impl ValueInfo {
    pub fn new<T: Reflect + ?Sized>() -> Self {
        Self {
            id: TypeIdentity::of::<T>(),
        }
    }

    /// The [`TypeIdentity`] of this value
    pub fn id(&self) -> &TypeIdentity {
        &self.id
    }
}

/// A container for compile-time info related to general Bevy's _dynamic_ types, including primitives.
///
/// This is functionally the same as [`ValueInfo`], however, semantically it refers to dynamic
/// types such as [`DynamicStruct`], [`DynamicTuple`], [`DynamicList`], etc.
///
/// [`DynamicStruct`]: crate::DynamicStruct
/// [`DynamicTuple`]: crate::DynamicTuple
/// [`DynamicList`]: crate::DynamicList
#[derive(Debug, Clone)]
pub struct DynamicInfo {
    id: TypeIdentity,
}

impl DynamicInfo {
    pub fn new<T: Reflect>() -> Self {
        Self {
            id: TypeIdentity::of::<T>(),
        }
    }

    /// The [`TypeIdentity`] of this dynamic value
    pub fn id(&self) -> &TypeIdentity {
        &self.id
    }
}

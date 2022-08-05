use crate::{
    ArrayInfo, EnumInfo, ListInfo, MapInfo, Reflect, StructInfo, TupleInfo, TupleStructInfo,
};
use std::any::{Any, TypeId};

/// A static accessor to compile-time type information.
///
/// This trait is automatically implemented by the `#[derive(Reflect)]` macro
/// and allows type information to be processed without an instance of that type.
///
/// # Implementing
///
/// While it is recommended to leave implementing this trait to the `#[derive(Reflect)]` macro,
/// it is possible to implement this trait manually. If a manual implementation is needed,
/// you _must_ ensure that the information you provide is correct, otherwise various systems that
/// rely on this trait may fail in unexpected ways.
///
/// Implementors may have difficulty in generating a reference to [`TypeInfo`] with a static
/// lifetime. Luckily, this crate comes with some [utility] structs, to make generating these
/// statics much simpler.
///
/// # Example
///
/// ```
/// # use std::any::Any;
/// # use bevy_reflect::{NamedField, Reflect, ReflectMut, ReflectRef, StructInfo, TypeInfo, ValueInfo};
/// # use bevy_reflect::utility::NonGenericTypeInfoCell;
/// use bevy_reflect::Typed;
///
/// struct MyStruct {
///   foo: usize,
///   bar: (f32, f32)
/// }
///
/// impl Typed for MyStruct {
///   fn type_info() -> &'static TypeInfo {
///     static CELL: NonGenericTypeInfoCell = NonGenericTypeInfoCell::new();
///     CELL.get_or_set(|| {
///       let fields = [
///         NamedField::new::<usize, _>("foo"),
///         NamedField::new::<(f32, f32), _>("bar"),
///       ];
///       let info = StructInfo::new::<Self>(&fields);
///       TypeInfo::Struct(info)
///     })
///   }
/// }
///
/// #
/// # impl Reflect for MyStruct {
/// #   fn type_name(&self) -> &str { todo!() }
/// #   fn get_type_info(&self) -> &'static TypeInfo { todo!() }
/// #   fn into_any(self: Box<Self>) -> Box<dyn Any> { todo!() }
/// #   fn as_any(&self) -> &dyn Any { todo!() }
/// #   fn as_any_mut(&mut self) -> &mut dyn Any { todo!() }
/// #   fn as_reflect(&self) -> &dyn Reflect { todo!() }
/// #   fn as_reflect_mut(&mut self) -> &mut dyn Reflect { todo!() }
/// #   fn apply(&mut self, value: &dyn Reflect) { todo!() }
/// #   fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> { todo!() }
/// #   fn reflect_ref(&self) -> ReflectRef { todo!() }
/// #   fn reflect_mut(&mut self) -> ReflectMut { todo!() }
/// #   fn clone_value(&self) -> Box<dyn Reflect> { todo!() }
/// # }
/// ```
///
/// [utility]: crate::utility
pub trait Typed: Reflect {
    /// Returns the compile-time [info] for the underlying type.
    ///
    /// [info]: TypeInfo
    fn type_info() -> &'static TypeInfo;
}

/// Compile-time type information for various reflected types.
///
/// Generally, for any given type, this value can be retrieved one of three ways:
///
/// 1. [`Typed::type_info`]
/// 2. [`Reflect::get_type_info`]
/// 3. [`TypeRegistry::get_type_info`]
///
/// Each return a static reference to [`TypeInfo`], but they all have their own use cases.
/// For example, if you know the type at compile time, [`Typed::type_info`] is probably
/// the simplest. If all you have is a `dyn Reflect`, you'll probably want [`Reflect::get_type_info`].
/// Lastly, if all you have is a [`TypeId`] or [type name], you will need to go through
/// [`TypeRegistry::get_type_info`].
///
/// You may also opt to use [`TypeRegistry::get_type_info`] in place of the other methods simply because
/// it can be more performant. This is because those other methods may require attaining a lock on
/// the static [`TypeInfo`], while the registry simply checks a map.
///
/// [`Reflect::get_type_info`]: crate::Reflect::get_type_info
/// [`TypeRegistry::get_type_info`]: crate::TypeRegistry::get_type_info
/// [`TypeId`]: std::any::TypeId
/// [type name]: std::any::type_name
#[derive(Debug, Clone)]
pub enum TypeInfo {
    Struct(StructInfo),
    TupleStruct(TupleStructInfo),
    Tuple(TupleInfo),
    List(ListInfo),
    Array(ArrayInfo),
    Map(MapInfo),
    Enum(EnumInfo),
    Value(ValueInfo),
    /// Type information for "dynamic" types whose metadata can't be known at compile-time.
    ///
    /// This includes structs like [`DynamicStruct`](crate::DynamicStruct) and [`DynamicList`](crate::DynamicList).
    Dynamic(DynamicInfo),
}

impl TypeInfo {
    /// The [`TypeId`] of the underlying type.
    pub fn type_id(&self) -> TypeId {
        match self {
            Self::Struct(info) => info.type_id(),
            Self::TupleStruct(info) => info.type_id(),
            Self::Tuple(info) => info.type_id(),
            Self::List(info) => info.type_id(),
            Self::Array(info) => info.type_id(),
            Self::Map(info) => info.type_id(),
            Self::Enum(info) => info.type_id(),
            Self::Value(info) => info.type_id(),
            Self::Dynamic(info) => info.type_id(),
        }
    }

    /// The [name] of the underlying type.
    ///
    /// [name]: std::any::type_name
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Struct(info) => info.type_name(),
            Self::TupleStruct(info) => info.type_name(),
            Self::Tuple(info) => info.type_name(),
            Self::List(info) => info.type_name(),
            Self::Array(info) => info.type_name(),
            Self::Map(info) => info.type_name(),
            Self::Enum(info) => info.type_name(),
            Self::Value(info) => info.type_name(),
            Self::Dynamic(info) => info.type_name(),
        }
    }

    /// Check if the given type matches the underlying type.
    pub fn is<T: Any>(&self) -> bool {
        TypeId::of::<T>() == self.type_id()
    }
}

/// A container for compile-time info related to general value types, including primitives.
///
/// This typically represents a type which cannot be broken down any further. This is often
/// due to technical reasons (or by definition), but it can also be a purposeful choice.
///
/// For example, [`i32`] cannot be broken down any further, so it is represented by a [`ValueInfo`].
/// And while [`String`] itself is a struct, it's fields are private, so we don't really treat
/// it _as_ a struct. It therefore makes more sense to represent it as a [`ValueInfo`].
#[derive(Debug, Clone)]
pub struct ValueInfo {
    type_name: &'static str,
    type_id: TypeId,
}

impl ValueInfo {
    pub fn new<T: Reflect + ?Sized>() -> Self {
        Self {
            type_name: std::any::type_name::<T>(),
            type_id: TypeId::of::<T>(),
        }
    }

    /// The [type name] of the value.
    ///
    /// [type name]: std::any::type_name
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// The [`TypeId`] of the value.
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Check if the given type matches the value type.
    pub fn is<T: Any>(&self) -> bool {
        TypeId::of::<T>() == self.type_id
    }
}

/// A container for compile-time info related to Bevy's _dynamic_ types, including primitives.
///
/// This is functionally the same as [`ValueInfo`], however, semantically it refers to dynamic
/// types such as [`DynamicStruct`], [`DynamicTuple`], [`DynamicList`], etc.
///
/// [`DynamicStruct`]: crate::DynamicStruct
/// [`DynamicTuple`]: crate::DynamicTuple
/// [`DynamicList`]: crate::DynamicList
#[derive(Debug, Clone)]
pub struct DynamicInfo {
    type_name: &'static str,
    type_id: TypeId,
}

impl DynamicInfo {
    pub fn new<T: Reflect>() -> Self {
        Self {
            type_name: std::any::type_name::<T>(),
            type_id: TypeId::of::<T>(),
        }
    }

    /// The [type name] of the dynamic value.
    ///
    /// [type name]: std::any::type_name
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// The [`TypeId`] of the dynamic value.
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Check if the given type matches the dynamic value type.
    pub fn is<T: Any>(&self) -> bool {
        TypeId::of::<T>() == self.type_id
    }
}

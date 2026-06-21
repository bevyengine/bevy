use crate::{
    array::ArrayInfo, enums::EnumInfo, generics::impl_generic_info_methods, list::ListInfo,
    map::MapInfo, set::SetInfo, structs::StructInfo, tuple::TupleInfo,
    tuple_struct::TupleStructInfo, OpaqueInfo, ReflectKind, Type, TypePathTable,
};
use core::any::{Any, TypeId};

/// Compile-time type information for various reflected types.
///
/// Generally, for any given type, this value can be retrieved in one of four ways:
///
/// 1. [`Typed::type_info`]
/// 2. [`DynamicTyped::reflect_type_info`]
/// 3. [`PartialReflect::get_represented_type_info`]
/// 4. [`TypeRegistry::get_type_info`]
///
/// Each returns a static reference to [`TypeInfo`], but they all have their own use cases.
/// For example, if you know the type at compile time, [`Typed::type_info`] is probably
/// the simplest. If you have a `dyn Reflect` you can use [`DynamicTyped::reflect_type_info`].
/// If all you have is a `dyn PartialReflect`, you'll probably want [`PartialReflect::get_represented_type_info`].
/// Lastly, if all you have is a [`TypeId`] or [type path], you will need to go through
/// [`TypeRegistry::get_type_info`].
///
/// You may also opt to use [`TypeRegistry::get_type_info`] in place of the other methods simply because
/// it can be more performant. This is because those other methods may require attaining a lock on
/// the static [`TypeInfo`], while the registry simply checks a map.
///
/// [`Typed::type_info`]: crate::info::Typed::type_info
/// [`DynamicTyped::reflect_type_info`]: crate::info::DynamicTyped::reflect_type_info
/// [`TypeRegistry::get_type_info`]: crate::TypeRegistry::get_type_info
/// [`PartialReflect::get_represented_type_info`]: crate::PartialReflect::get_represented_type_info
/// [type path]: crate::type_path::TypePath::type_path
#[derive(Debug, Clone)]
pub enum TypeInfo {
    /// Type information for a [struct-like] type.
    ///
    /// [struct-like]: crate::structs::Struct
    Struct(StructInfo),
    /// Type information for a [tuple-struct-like] type.
    ///
    /// [tuple-struct-like]: crate::tuple_struct::TupleStruct
    TupleStruct(TupleStructInfo),
    /// Type information for a [tuple-like] type.
    ///
    /// [tuple-like]: crate::tuple::Tuple
    Tuple(TupleInfo),
    /// Type information for a [list-like] type.
    ///
    /// [list-like]: crate::list::List
    List(ListInfo),
    /// Type information for an [array-like] type.
    ///
    /// [array-like]: crate::array::Array
    Array(ArrayInfo),
    /// Type information for a [map-like] type.
    ///
    /// [map-like]: crate::map::Map
    Map(MapInfo),
    /// Type information for a [set-like] type.
    ///
    /// [set-like]: crate::set::Set
    Set(SetInfo),
    /// Type information for an [enum-like] type.
    ///
    /// [enum-like]: crate::enums::Enum
    Enum(EnumInfo),
    /// Type information for an opaque type - see the [`OpaqueInfo`] docs for
    /// a discussion of opaque types.
    Opaque(OpaqueInfo),
}

impl TypeInfo {
    /// The underlying Rust [type].
    ///
    /// [type]: Type
    pub fn ty(&self) -> &Type {
        match self {
            Self::Struct(info) => info.ty(),
            Self::TupleStruct(info) => info.ty(),
            Self::Tuple(info) => info.ty(),
            Self::List(info) => info.ty(),
            Self::Array(info) => info.ty(),
            Self::Map(info) => info.ty(),
            Self::Set(info) => info.ty(),
            Self::Enum(info) => info.ty(),
            Self::Opaque(info) => info.ty(),
        }
    }

    /// The [`TypeId`] of the underlying type.
    #[inline]
    pub fn type_id(&self) -> TypeId {
        self.ty().id()
    }

    /// A representation of the type path of the underlying type.
    ///
    /// Provides dynamic access to all methods on [`TypePath`].
    ///
    /// [`TypePath`]: crate::type_path::TypePath
    pub fn type_path_table(&self) -> &TypePathTable {
        self.ty().type_path_table()
    }

    /// The [stable, full type path] of the underlying type.
    ///
    /// Use [`type_path_table`] if you need access to the other methods on [`TypePath`].
    ///
    /// [stable, full type path]: crate::type_path::TypePath
    /// [`type_path_table`]: Self::type_path_table
    /// [`TypePath`]: crate::type_path::TypePath
    pub fn type_path(&self) -> &'static str {
        self.ty().path()
    }

    /// Check if the given type matches this one.
    ///
    /// This only compares the [`TypeId`] of the types
    /// and does not verify they share the same [`TypePath`]
    /// (though it implies they do).
    ///
    /// [`TypePath`]: crate::type_path::TypePath
    pub fn is<T: Any>(&self) -> bool {
        self.ty().is::<T>()
    }

    /// The docstring of the underlying type, if any.
    #[cfg(feature = "reflect_documentation")]
    pub fn docs(&self) -> Option<&str> {
        match self {
            Self::Struct(info) => info.docs(),
            Self::TupleStruct(info) => info.docs(),
            Self::Tuple(info) => info.docs(),
            Self::List(info) => info.docs(),
            Self::Array(info) => info.docs(),
            Self::Map(info) => info.docs(),
            Self::Set(info) => info.docs(),
            Self::Enum(info) => info.docs(),
            Self::Opaque(info) => info.docs(),
        }
    }

    /// Returns the [kind] of this `TypeInfo`.
    ///
    /// [kind]: ReflectKind
    pub fn kind(&self) -> ReflectKind {
        match self {
            Self::Struct(_) => ReflectKind::Struct,
            Self::TupleStruct(_) => ReflectKind::TupleStruct,
            Self::Tuple(_) => ReflectKind::Tuple,
            Self::List(_) => ReflectKind::List,
            Self::Array(_) => ReflectKind::Array,
            Self::Map(_) => ReflectKind::Map,
            Self::Set(_) => ReflectKind::Set,
            Self::Enum(_) => ReflectKind::Enum,
            Self::Opaque(_) => ReflectKind::Opaque,
        }
    }

    impl_generic_info_methods!(self => {
        match self {
            Self::Struct(info) => info.generics(),
            Self::TupleStruct(info) => info.generics(),
            Self::Tuple(info) => info.generics(),
            Self::List(info) => info.generics(),
            Self::Array(info) => info.generics(),
            Self::Map(info) => info.generics(),
            Self::Set(info) => info.generics(),
            Self::Enum(info) => info.generics(),
            Self::Opaque(info) => info.generics(),
        }
    });
}

macro_rules! impl_cast_method {
    ($name:ident : $kind:ident => $info:ident) => {
        #[doc = concat!("Attempts a cast to [`", stringify!($info), "`].")]
        #[doc = concat!("\n\nReturns an error if `self` is not [`TypeInfo::", stringify!($kind), "`].")]
        pub fn $name(&self) -> Result<&$info, $crate::info::error::TypeInfoError> {
            match self {
                Self::$kind(info) => Ok(info),
                _ => Err($crate::info::error::TypeInfoError::KindMismatch {
                    expected: ReflectKind::$kind,
                    received: self.kind(),
                }),
            }
        }
    };
}

/// Conversion convenience methods for [`TypeInfo`].
impl TypeInfo {
    impl_cast_method!(as_struct: Struct => StructInfo);
    impl_cast_method!(as_tuple_struct: TupleStruct => TupleStructInfo);
    impl_cast_method!(as_tuple: Tuple => TupleInfo);
    impl_cast_method!(as_list: List => ListInfo);
    impl_cast_method!(as_array: Array => ArrayInfo);
    impl_cast_method!(as_map: Map => MapInfo);
    impl_cast_method!(as_set: Set => SetInfo);
    impl_cast_method!(as_enum: Enum => EnumInfo);
    impl_cast_method!(as_opaque: Opaque => OpaqueInfo);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::info::typed::Typed;
    use crate::TypeInfoError;
    use alloc::vec::Vec;
    use bevy_platform::collections::HashSet;

    #[test]
    fn should_return_error_on_invalid_cast() {
        let info = <Vec<i32> as Typed>::type_info();
        assert!(matches!(
            info.as_struct(),
            Err(TypeInfoError::KindMismatch {
                expected: ReflectKind::Struct,
                received: ReflectKind::List
            })
        ));
    }

    #[test]
    fn should_cast_to_set() {
        let info = <HashSet<u64> as Typed>::type_info();
        assert!(info.as_set().is_ok());
    }
}

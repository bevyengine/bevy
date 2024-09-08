use crate::{
    ArrayInfo, DynamicArray, DynamicEnum, DynamicList, DynamicMap, DynamicStruct, DynamicTuple,
    DynamicTupleStruct, EnumInfo, ListInfo, MapInfo, PartialReflect, Reflect, ReflectKind, SetInfo,
    StructInfo, TupleInfo, TupleStructInfo, TypePath, TypePathTable,
};
use core::fmt::Formatter;
use std::any::{Any, TypeId};
use std::fmt::Debug;
use std::hash::Hash;
use thiserror::Error;

/// A static accessor to compile-time type information.
///
/// This trait is automatically implemented by the [`#[derive(Reflect)]`](derive@crate::Reflect) macro
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
/// # use bevy_reflect::{DynamicTypePath, NamedField, PartialReflect, Reflect, ReflectMut, ReflectOwned, ReflectRef, StructInfo, TypeInfo, TypePath, ValueInfo, ApplyError};
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
///         NamedField::new::<usize >("foo"),
///         NamedField::new::<(f32, f32) >("bar"),
///       ];
///       let info = StructInfo::new::<Self>(&fields);
///       TypeInfo::Struct(info)
///     })
///   }
/// }
///
/// # impl TypePath for MyStruct {
/// #     fn type_path() -> &'static str { todo!() }
/// #     fn short_type_path() -> &'static str { todo!() }
/// # }
/// # impl PartialReflect for MyStruct {
/// #     fn get_represented_type_info(&self) -> Option<&'static TypeInfo> { todo!() }
/// #     fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect> { todo!() }
/// #     fn as_partial_reflect(&self) -> &dyn PartialReflect { todo!() }
/// #     fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect { todo!() }
/// #     fn try_into_reflect(self: Box<Self>) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>> { todo!() }
/// #     fn try_as_reflect(&self) -> Option<&dyn Reflect> { todo!() }
/// #     fn try_as_reflect_mut(&mut self) -> Option<&mut dyn Reflect> { todo!() }
/// #     fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> { todo!() }
/// #     fn reflect_ref(&self) -> ReflectRef { todo!() }
/// #     fn reflect_mut(&mut self) -> ReflectMut { todo!() }
/// #     fn reflect_owned(self: Box<Self>) -> ReflectOwned { todo!() }
/// #     fn clone_value(&self) -> Box<dyn PartialReflect> { todo!() }
/// # }
/// # impl Reflect for MyStruct {
/// #     fn into_any(self: Box<Self>) -> Box<dyn Any> { todo!() }
/// #     fn as_any(&self) -> &dyn Any { todo!() }
/// #     fn as_any_mut(&mut self) -> &mut dyn Any { todo!() }
/// #     fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> { todo!() }
/// #     fn as_reflect(&self) -> &dyn Reflect { todo!() }
/// #     fn as_reflect_mut(&mut self) -> &mut dyn Reflect { todo!() }
/// #     fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> { todo!() }
/// # }
/// ```
///
/// [utility]: crate::utility
#[diagnostic::on_unimplemented(
    message = "`{Self}` can not provide type information through reflection",
    note = "consider annotating `{Self}` with `#[derive(Reflect)]`"
)]
pub trait Typed: Reflect + TypePath {
    /// Returns the compile-time [info] for the underlying type.
    ///
    /// [info]: TypeInfo
    fn type_info() -> &'static TypeInfo;
}

/// A wrapper trait around [`Typed`].
///
/// This trait is used to provide a way to get compile-time type information for types that
/// do implement `Typed` while also allowing for types that do not implement `Typed` to be used.
/// It's used instead of `Typed` directly to avoid making dynamic types also
/// implement `Typed` in order to be used as active fields.
///
/// This trait has a blanket implementation for all types that implement `Typed`
/// and manual implementations for all dynamic types (which simply return `None`).
#[doc(hidden)]
pub trait MaybeTyped: PartialReflect {
    /// Returns the compile-time [info] for the underlying type, if it exists.
    ///
    /// [info]: TypeInfo
    fn maybe_type_info() -> Option<&'static TypeInfo> {
        None
    }
}

impl<T: Typed> MaybeTyped for T {
    fn maybe_type_info() -> Option<&'static TypeInfo> {
        Some(T::type_info())
    }
}

impl MaybeTyped for DynamicEnum {}

impl MaybeTyped for DynamicTupleStruct {}

impl MaybeTyped for DynamicStruct {}

impl MaybeTyped for DynamicMap {}

impl MaybeTyped for DynamicList {}

impl MaybeTyped for DynamicArray {}

impl MaybeTyped for DynamicTuple {}

/// A [`TypeInfo`]-specific error.
#[derive(Debug, Error)]
pub enum TypeInfoError {
    /// Caused when a type was expected to be of a certain [kind], but was not.
    ///
    /// [kind]: ReflectKind
    #[error("kind mismatch: expected {expected:?}, received {received:?}")]
    KindMismatch {
        expected: ReflectKind,
        received: ReflectKind,
    },
}

/// Compile-time type information for various reflected types.
///
/// Generally, for any given type, this value can be retrieved one of three ways:
///
/// 1. [`Typed::type_info`]
/// 2. [`PartialReflect::get_represented_type_info`]
/// 3. [`TypeRegistry::get_type_info`]
///
/// Each return a static reference to [`TypeInfo`], but they all have their own use cases.
/// For example, if you know the type at compile time, [`Typed::type_info`] is probably
/// the simplest. If all you have is a `dyn PartialReflect`, you'll probably want [`PartialReflect::get_represented_type_info`].
/// Lastly, if all you have is a [`TypeId`] or [type path], you will need to go through
/// [`TypeRegistry::get_type_info`].
///
/// You may also opt to use [`TypeRegistry::get_type_info`] in place of the other methods simply because
/// it can be more performant. This is because those other methods may require attaining a lock on
/// the static [`TypeInfo`], while the registry simply checks a map.
///
/// [`TypeRegistry::get_type_info`]: crate::TypeRegistry::get_type_info
/// [`PartialReflect::get_represented_type_info`]: crate::PartialReflect::get_represented_type_info
/// [type path]: TypePath::type_path
#[derive(Debug, Clone)]
pub enum TypeInfo {
    Struct(StructInfo),
    TupleStruct(TupleStructInfo),
    Tuple(TupleInfo),
    List(ListInfo),
    Array(ArrayInfo),
    Map(MapInfo),
    Set(SetInfo),
    Enum(EnumInfo),
    Value(ValueInfo),
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
            Self::Value(info) => info.ty(),
        }
    }

    /// The [`TypeId`] of the underlying type.
    pub fn type_id(&self) -> TypeId {
        self.ty().id()
    }

    /// A representation of the type path of the underlying type.
    ///
    /// Provides dynamic access to all methods on [`TypePath`].
    pub fn type_path_table(&self) -> &TypePathTable {
        self.ty().type_path_table()
    }

    /// The [stable, full type path] of the underlying type.
    ///
    /// Use [`type_path_table`] if you need access to the other methods on [`TypePath`].
    ///
    /// [stable, full type path]: TypePath
    /// [`type_path_table`]: Self::type_path_table
    pub fn type_path(&self) -> &'static str {
        self.ty().path()
    }

    /// Check if the given type matches this one.
    ///
    /// This only compares the [`TypeId`] of the types
    /// and does not verify they share the same [`TypePath`]
    /// (though it implies they do).
    pub fn is<T: Any>(&self) -> bool {
        self.ty().is::<T>()
    }

    /// The docstring of the underlying type, if any.
    #[cfg(feature = "documentation")]
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
            Self::Value(info) => info.docs(),
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
            Self::Value(_) => ReflectKind::Value,
        }
    }
}

macro_rules! impl_cast_method {
    ($name:ident : $kind:ident => $info:ident) => {
        #[doc = concat!("Attempts a cast to [`", stringify!($info), "`].")]
        #[doc = concat!("\n\nReturns an error if `self` is not [`TypeInfo::", stringify!($kind), "`].")]
        pub fn $name(&self) -> Result<&$info, TypeInfoError> {
            match self {
                Self::$kind(info) => Ok(info),
                _ => Err(TypeInfoError::KindMismatch {
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
    impl_cast_method!(as_enum: Enum => EnumInfo);
    impl_cast_method!(as_value: Value => ValueInfo);
}

/// The base representation of a Rust type.
///
/// When possible, it is recommended to use [`&'static TypeInfo`] instead of this
/// as it provides more information as well as being smaller
/// (since a reference only takes the same number of bytes as a `usize`).
///
/// However, where a static reference to [`TypeInfo`] is not possible,
/// such as with trait objects and other types that can't implement [`Typed`],
/// this type can be used instead.
///
/// It only requires that the type implements [`TypePath`].
///
/// And unlike [`TypeInfo`], this type implements [`Copy`], [`Eq`], and [`Hash`],
/// making it useful as a key type.
///
/// It's especially helpful when compared to [`TypeId`] as it can provide the
/// actual [type path] when debugging, while still having the same performance
/// as hashing/comparing [`TypeId`] directly—at the cost of a little more memory.
///
/// # Examples
///
/// ```
/// use bevy_reflect::{Type, TypePath};
///
/// fn assert_char<T: ?Sized + TypePath>(t: &T) -> Result<(), String> {
///     let ty = Type::of::<T>();
///     if Type::of::<char>() == ty {
///         Ok(())
///     } else {
///         Err(format!("expected `char`, got `{}`", ty.path()))
///     }
/// }
///
/// assert_eq!(
///     assert_char(&'a'),
///     Ok(())
/// );
/// assert_eq!(
///     assert_char(&String::from("Hello, world!")),
///     Err(String::from("expected `char`, got `alloc::string::String`"))
/// );
/// ```
///
/// [`&'static TypeInfo`]: TypeInfo
#[derive(Copy, Clone)]
pub struct Type {
    type_path_table: TypePathTable,
    type_id: TypeId,
}

impl Type {
    /// Create a new [`Type`] from a type that implements [`TypePath`].
    pub fn of<T: TypePath + ?Sized>() -> Self {
        Self {
            type_path_table: TypePathTable::of::<T>(),
            type_id: TypeId::of::<T>(),
        }
    }

    /// Returns the [`TypeId`] of the type.
    pub fn id(&self) -> TypeId {
        self.type_id
    }

    /// See [`TypePath::type_path`].
    pub fn path(&self) -> &'static str {
        self.type_path_table.path()
    }

    /// See [`TypePath::short_type_path`].
    pub fn short_path(&self) -> &'static str {
        self.type_path_table.short_path()
    }

    /// See [`TypePath::type_ident`].
    pub fn ident(&self) -> Option<&'static str> {
        self.type_path_table.ident()
    }

    /// See [`TypePath::crate_name`].
    pub fn crate_name(&self) -> Option<&'static str> {
        self.type_path_table.crate_name()
    }

    /// See [`TypePath::module_path`].
    pub fn module_path(&self) -> Option<&'static str> {
        self.type_path_table.module_path()
    }

    /// A representation of the type path of this.
    ///
    /// Provides dynamic access to all methods on [`TypePath`].
    pub fn type_path_table(&self) -> &TypePathTable {
        &self.type_path_table
    }

    /// Check if the given type matches this one.
    ///
    /// This only compares the [`TypeId`] of the types
    /// and does not verify they share the same [`TypePath`]
    /// (though it implies they do).
    pub fn is<T: Any>(&self) -> bool {
        TypeId::of::<T>() == self.type_id
    }
}

/// This implementation will only output the [type path] of the type.
///
/// If you need to include the [`TypeId`] in the output,
/// you can access it through [`Type::id`].
///
/// [type path]: TypePath
impl Debug for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.type_path_table.path())
    }
}

impl Eq for Type {}

/// This implementation purely relies on the [`TypeId`] of the type,
/// and not on the [type path].
///
/// [type path]: TypePath
impl PartialEq for Type {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id
    }
}

/// This implementation purely relies on the [`TypeId`] of the type,
/// and not on the [type path].
///
/// [type path]: TypePath
impl Hash for Type {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.type_id.hash(state);
    }
}

macro_rules! impl_type_methods {
    ($field:ident) => {
        /// The underlying Rust [type].
        ///
        /// [type]: crate::type_info::Type
        pub fn ty(&self) -> &$crate::type_info::Type {
            &self.$field
        }

        /// The [`TypeId`] of this type.
        ///
        /// [`TypeId`]: std::any::TypeId
        pub fn type_id(&self) -> ::std::any::TypeId {
            self.$field.id()
        }

        /// The [stable, full type path] of this type.
        ///
        /// Use [`type_path_table`] if you need access to the other methods on [`TypePath`].
        ///
        /// [stable, full type path]: TypePath
        /// [`type_path_table`]: Self::type_path_table
        pub fn type_path(&self) -> &'static str {
            self.$field.path()
        }

        /// A representation of the type path of this type.
        ///
        /// Provides dynamic access to all methods on [`TypePath`].
        ///
        /// [`TypePath`]: crate::type_path::TypePath
        pub fn type_path_table(&self) -> &$crate::type_path::TypePathTable {
            &self.$field.type_path_table()
        }

        /// Check if the given type matches this one.
        ///
        /// This only compares the [`TypeId`] of the types
        /// and does not verify they share the same [`TypePath`]
        /// (though it implies they do).
        ///
        /// [`TypeId`]: std::any::TypeId
        /// [`TypePath`]: crate::type_path::TypePath
        pub fn is<T: ::std::any::Any>(&self) -> bool {
            self.$field.is::<T>()
        }
    };
}

pub(crate) use impl_type_methods;

/// A container for compile-time info related to general value types, including primitives.
///
/// This typically represents a type which cannot be broken down any further. This is often
/// due to technical reasons (or by definition), but it can also be a purposeful choice.
///
/// For example, [`i32`] cannot be broken down any further, so it is represented by a [`ValueInfo`].
/// And while [`String`] itself is a struct, its fields are private, so we don't really treat
/// it _as_ a struct. It therefore makes more sense to represent it as a [`ValueInfo`].
#[derive(Debug, Clone)]
pub struct ValueInfo {
    ty: Type,
    #[cfg(feature = "documentation")]
    docs: Option<&'static str>,
}

impl ValueInfo {
    pub fn new<T: Reflect + TypePath + ?Sized>() -> Self {
        Self {
            ty: Type::of::<T>(),
            #[cfg(feature = "documentation")]
            docs: None,
        }
    }

    /// Sets the docstring for this value.
    #[cfg(feature = "documentation")]
    pub fn with_docs(self, doc: Option<&'static str>) -> Self {
        Self { docs: doc, ..self }
    }

    impl_type_methods!(ty);

    /// The docstring of this dynamic value, if any.
    #[cfg(feature = "documentation")]
    pub fn docs(&self) -> Option<&'static str> {
        self.docs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

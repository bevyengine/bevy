use crate::{
    ArrayInfo, DynamicArray, DynamicEnum, DynamicList, DynamicMap, DynamicStruct, DynamicTuple,
    DynamicTupleStruct, EnumInfo, Generics, ListInfo, MapInfo, PartialReflect, Reflect,
    ReflectKind, SetInfo, StructInfo, TupleInfo, TupleStructInfo, TypePath, TypePathTable,
};
use core::{
    any::{Any, TypeId},
    fmt::{Debug, Formatter},
    hash::Hash,
};
use thiserror::Error;

/// A static accessor to compile-time type information.
///
/// This trait is automatically implemented by the [`#[derive(Reflect)]`](derive@crate::Reflect) macro
/// and allows type information to be processed without an instance of that type.
///
/// If you need to use this trait as a generic bound along with other reflection traits,
/// for your convenience, consider using [`Reflectable`] instead.
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
/// # use core::any::Any;
/// # use bevy_reflect::{DynamicTypePath, NamedField, PartialReflect, Reflect, ReflectMut, ReflectOwned, ReflectRef, StructInfo, TypeInfo, TypePath, OpaqueInfo, ApplyError};
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
/// #     fn reflect_ref(&self) -> ReflectRef<'_> { todo!() }
/// #     fn reflect_mut(&mut self) -> ReflectMut<'_> { todo!() }
/// #     fn reflect_owned(self: Box<Self>) -> ReflectOwned { todo!() }
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
/// [`Reflectable`]: crate::Reflectable
/// [utility]: crate::utility
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not implement `Typed` so cannot provide static type information",
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
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not implement `Typed` so cannot provide static type information",
    note = "consider annotating `{Self}` with `#[derive(Reflect)]`"
)]
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

/// Dynamic dispatch for [`Typed`].
///
/// Since this is a supertrait of [`Reflect`] its methods can be called on a `dyn Reflect`.
///
/// [`Reflect`]: crate::Reflect
#[diagnostic::on_unimplemented(
    message = "`{Self}` can not provide dynamic type information through reflection",
    note = "consider annotating `{Self}` with `#[derive(Reflect)]`"
)]
pub trait DynamicTyped {
    /// See [`Typed::type_info`].
    fn reflect_type_info(&self) -> &'static TypeInfo;
}

impl<T: Typed> DynamicTyped for T {
    #[inline]
    fn reflect_type_info(&self) -> &'static TypeInfo {
        Self::type_info()
    }
}

/// A [`TypeInfo`]-specific error.
#[derive(Debug, Error)]
pub enum TypeInfoError {
    /// Caused when a type was expected to be of a certain [kind], but was not.
    ///
    /// [kind]: ReflectKind
    #[error("kind mismatch: expected {expected:?}, received {received:?}")]
    KindMismatch {
        /// Expected kind.
        expected: ReflectKind,
        /// Received kind.
        received: ReflectKind,
    },
}

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
/// [`TypeRegistry::get_type_info`]: crate::TypeRegistry::get_type_info
/// [`PartialReflect::get_represented_type_info`]: crate::PartialReflect::get_represented_type_info
/// [type path]: TypePath::type_path
#[derive(Debug, Clone)]
pub enum TypeInfo {
    /// Type information for a [struct-like] type.
    ///
    /// [struct-like]: crate::Struct
    Struct(StructInfo),
    /// Type information for a [tuple-struct-like] type.
    ///
    /// [tuple-struct-like]: crate::TupleStruct
    TupleStruct(TupleStructInfo),
    /// Type information for a [tuple-like] type.
    ///
    /// [tuple-like]: crate::Tuple
    Tuple(TupleInfo),
    /// Type information for a [list-like] type.
    ///
    /// [list-like]: crate::List
    List(ListInfo),
    /// Type information for an [array-like] type.
    ///
    /// [array-like]: crate::Array
    Array(ArrayInfo),
    /// Type information for a [map-like] type.
    ///
    /// [map-like]: crate::Map
    Map(MapInfo),
    /// Type information for a [set-like] type.
    ///
    /// [set-like]: crate::Set
    Set(SetInfo),
    /// Type information for an [enum-like] type.
    ///
    /// [enum-like]: crate::Enum
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
    impl_cast_method!(as_opaque: Opaque => OpaqueInfo);
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
    #[inline]
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
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.type_id.hash(state);
    }
}

macro_rules! impl_type_methods {
    // Generates the type methods based off a single field.
    ($field:ident) => {
        $crate::type_info::impl_type_methods!(self => {
            &self.$field
        });
    };
    // Generates the type methods based off a custom expression.
    ($self:ident => $expr:expr) => {
        /// The underlying Rust [type].
        ///
        /// [type]: crate::type_info::Type
        pub fn ty(&$self) -> &$crate::type_info::Type {
            $expr
        }

        /// The [`TypeId`] of this type.
        ///
        /// [`TypeId`]: core::any::TypeId
        pub fn type_id(&self) -> ::core::any::TypeId {
            self.ty().id()
        }

        /// The [stable, full type path] of this type.
        ///
        /// Use [`type_path_table`] if you need access to the other methods on [`TypePath`].
        ///
        /// [stable, full type path]: TypePath
        /// [`type_path_table`]: Self::type_path_table
        pub fn type_path(&self) -> &'static str {
            self.ty().path()
        }

        /// A representation of the type path of this type.
        ///
        /// Provides dynamic access to all methods on [`TypePath`].
        ///
        /// [`TypePath`]: crate::type_path::TypePath
        pub fn type_path_table(&self) -> &$crate::type_path::TypePathTable {
            &self.ty().type_path_table()
        }

        /// Check if the given type matches this one.
        ///
        /// This only compares the [`TypeId`] of the types
        /// and does not verify they share the same [`TypePath`]
        /// (though it implies they do).
        ///
        /// [`TypeId`]: core::any::TypeId
        /// [`TypePath`]: crate::type_path::TypePath
        pub fn is<T: ::core::any::Any>(&self) -> bool {
            self.ty().is::<T>()
        }
    };
}

use crate::generics::impl_generic_info_methods;
pub(crate) use impl_type_methods;

/// A container for compile-time info related to reflection-opaque types, including primitives.
///
/// This typically represents a type which cannot be broken down any further. This is often
/// due to technical reasons (or by definition), but it can also be a purposeful choice.
///
/// For example, [`i32`] cannot be broken down any further, so it is represented by an [`OpaqueInfo`].
/// And while [`String`] itself is a struct, its fields are private, so we don't really treat
/// it _as_ a struct. It therefore makes more sense to represent it as an [`OpaqueInfo`].
///
/// [`String`]: alloc::string::String
#[derive(Debug, Clone)]
pub struct OpaqueInfo {
    ty: Type,
    generics: Generics,
    #[cfg(feature = "documentation")]
    docs: Option<&'static str>,
}

impl OpaqueInfo {
    /// Creates a new [`OpaqueInfo`].
    pub fn new<T: Reflect + TypePath + ?Sized>() -> Self {
        Self {
            ty: Type::of::<T>(),
            generics: Generics::new(),
            #[cfg(feature = "documentation")]
            docs: None,
        }
    }

    /// Sets the docstring for this type.
    #[cfg(feature = "documentation")]
    pub fn with_docs(self, doc: Option<&'static str>) -> Self {
        Self { docs: doc, ..self }
    }

    impl_type_methods!(ty);

    /// The docstring of this dynamic type, if any.
    #[cfg(feature = "documentation")]
    pub fn docs(&self) -> Option<&'static str> {
        self.docs
    }

    impl_generic_info_methods!(generics);
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

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

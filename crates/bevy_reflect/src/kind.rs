#[cfg(feature = "functions")]
use crate::func::Function;
use crate::{Array, Enum, List, Map, PartialReflect, Set, Struct, Tuple, TupleStruct};
use thiserror::Error;

/// A zero-sized enumeration of the "kinds" of a reflected type.
///
/// Each kind corresponds to a specific reflection trait,
/// such as [`Struct`] or [`List`],
/// which itself corresponds to the kind or structure of a type.
///
/// A [`ReflectKind`] is obtained via [`PartialReflect::reflect_kind`],
/// or via [`ReflectRef::kind`],[`ReflectMut::kind`] or [`ReflectOwned::kind`].
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ReflectKind {
    /// A [struct-like] type.
    ///
    /// [struct-like]: Struct
    Struct,
    /// A [tuple-struct-like] type.
    ///
    /// [tuple-struct-like]: TupleStruct
    TupleStruct,
    /// A [tuple-like] type.
    ///
    /// [tuple-like]: Tuple
    Tuple,
    /// A [list-like] type.
    ///
    /// [list-like]: List
    List,
    /// An [array-like] type.
    ///
    /// [array-like]: Array
    Array,
    /// A [map-like] type.
    ///
    /// [map-like]: Map
    Map,
    /// A [set-like] type.
    ///
    /// [set-like]: Set
    Set,
    /// An [enum-like] type.
    ///
    /// [enum-like]: Enum
    Enum,
    /// A [function-like] type.
    ///
    /// [function-like]: Function
    #[cfg(feature = "functions")]
    Function,
    /// A value-like type.
    ///
    /// This most often represents a primitive or opaque type,
    /// where it is not possible, difficult, or not useful to reflect the type further.
    ///
    /// For example, `u32` and `String` are examples of value-like types.
    /// Additionally, any type that derives [`Reflect`] with the `#[reflect_value]` attribute
    /// will be considered a value-like type.
    ///
    /// [`Reflect`]: crate::Reflect
    Value,
}

impl std::fmt::Display for ReflectKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReflectKind::Struct => f.pad("struct"),
            ReflectKind::TupleStruct => f.pad("tuple struct"),
            ReflectKind::Tuple => f.pad("tuple"),
            ReflectKind::List => f.pad("list"),
            ReflectKind::Array => f.pad("array"),
            ReflectKind::Map => f.pad("map"),
            ReflectKind::Set => f.pad("set"),
            ReflectKind::Enum => f.pad("enum"),
            #[cfg(feature = "functions")]
            ReflectKind::Function => f.pad("function"),
            ReflectKind::Value => f.pad("value"),
        }
    }
}

macro_rules! impl_reflect_kind_conversions {
    ($name:ident$(<$lifetime:lifetime>)?) => {
        impl $name$(<$lifetime>)? {
            /// Returns the "kind" of this reflected type without any information.
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
                    #[cfg(feature = "functions")]
                    Self::Function(_) => ReflectKind::Function,
                    Self::Value(_) => ReflectKind::Value,
                }
            }
        }

        impl From<$name$(<$lifetime>)?> for ReflectKind {
            fn from(value: $name) -> Self {
                match value {
                    $name::Struct(_) => Self::Struct,
                    $name::TupleStruct(_) => Self::TupleStruct,
                    $name::Tuple(_) => Self::Tuple,
                    $name::List(_) => Self::List,
                    $name::Array(_) => Self::Array,
                    $name::Map(_) => Self::Map,
                    $name::Set(_) => Self::Set,
                    $name::Enum(_) => Self::Enum,
                    #[cfg(feature = "functions")]
                    $name::Function(_) => Self::Function,
                    $name::Value(_) => Self::Value,
                }
            }
        }
    };
}

/// Caused when a type was expected to be of a certain [kind], but was not.
///
/// [kind]: ReflectKind
#[derive(Debug, Error)]
#[error("kind mismatch: expected {expected:?}, received {received:?}")]
pub struct ReflectKindMismatchError {
    pub expected: ReflectKind,
    pub received: ReflectKind,
}

macro_rules! impl_cast_method {
    ($name:ident : Value => $retval:ty) => {
        #[doc = "Attempts a cast to a [`PartialReflect`] trait object."]
        #[doc = "\n\nReturns an error if `self` is not the [`Self::Value`] variant."]
        pub fn $name(self) -> Result<$retval, ReflectKindMismatchError> {
            match self {
                Self::Value(value) => Ok(value),
                _ => Err(ReflectKindMismatchError {
                    expected: ReflectKind::Value,
                    received: self.kind(),
                }),
            }
        }
    };
    ($name:ident : $kind:ident => $retval:ty) => {
        #[doc = concat!("Attempts a cast to a [`", stringify!($kind), "`] trait object.")]
        #[doc = concat!("\n\nReturns an error if `self` is not the [`Self::", stringify!($kind), "`] variant.")]
        pub fn $name(self) -> Result<$retval, ReflectKindMismatchError> {
            match self {
                Self::$kind(value) => Ok(value),
                _ => Err(ReflectKindMismatchError {
                    expected: ReflectKind::$kind,
                    received: self.kind(),
                }),
            }
        }
    };
}

/// An immutable enumeration of ["kinds"] of a reflected type.
///
/// Each variant contains a trait object with methods specific to a kind of
/// type.
///
/// A [`ReflectRef`] is obtained via [`PartialReflect::reflect_ref`].
///
/// ["kinds"]: ReflectKind
pub enum ReflectRef<'a> {
    Struct(&'a dyn Struct),
    TupleStruct(&'a dyn TupleStruct),
    Tuple(&'a dyn Tuple),
    List(&'a dyn List),
    Array(&'a dyn Array),
    Map(&'a dyn Map),
    Set(&'a dyn Set),
    Enum(&'a dyn Enum),
    #[cfg(feature = "functions")]
    Function(&'a dyn Function),
    Value(&'a dyn PartialReflect),
}
impl_reflect_kind_conversions!(ReflectRef<'_>);

impl<'a> ReflectRef<'a> {
    impl_cast_method!(as_struct: Struct => &'a dyn Struct);
    impl_cast_method!(as_tuple_struct: TupleStruct => &'a dyn TupleStruct);
    impl_cast_method!(as_tuple: Tuple => &'a dyn Tuple);
    impl_cast_method!(as_list: List => &'a dyn List);
    impl_cast_method!(as_array: Array => &'a dyn Array);
    impl_cast_method!(as_map: Map => &'a dyn Map);
    impl_cast_method!(as_set: Set => &'a dyn Set);
    impl_cast_method!(as_enum: Enum => &'a dyn Enum);
    impl_cast_method!(as_value: Value => &'a dyn PartialReflect);
}

/// A mutable enumeration of ["kinds"] of a reflected type.
///
/// Each variant contains a trait object with methods specific to a kind of
/// type.
///
/// A [`ReflectMut`] is obtained via [`PartialReflect::reflect_mut`].
///
/// ["kinds"]: ReflectKind
pub enum ReflectMut<'a> {
    Struct(&'a mut dyn Struct),
    TupleStruct(&'a mut dyn TupleStruct),
    Tuple(&'a mut dyn Tuple),
    List(&'a mut dyn List),
    Array(&'a mut dyn Array),
    Map(&'a mut dyn Map),
    Set(&'a mut dyn Set),
    Enum(&'a mut dyn Enum),
    #[cfg(feature = "functions")]
    Function(&'a mut dyn Function),
    Value(&'a mut dyn PartialReflect),
}
impl_reflect_kind_conversions!(ReflectMut<'_>);

impl<'a> ReflectMut<'a> {
    impl_cast_method!(as_struct: Struct => &'a mut dyn Struct);
    impl_cast_method!(as_tuple_struct: TupleStruct => &'a mut dyn TupleStruct);
    impl_cast_method!(as_tuple: Tuple => &'a mut dyn Tuple);
    impl_cast_method!(as_list: List => &'a mut dyn List);
    impl_cast_method!(as_array: Array => &'a mut dyn Array);
    impl_cast_method!(as_map: Map => &'a mut dyn Map);
    impl_cast_method!(as_set: Set => &'a mut dyn Set);
    impl_cast_method!(as_enum: Enum => &'a mut dyn Enum);
    impl_cast_method!(as_value: Value => &'a mut dyn PartialReflect);
}

/// An owned enumeration of ["kinds"] of a reflected type.
///
/// Each variant contains a trait object with methods specific to a kind of
/// type.
///
/// A [`ReflectOwned`] is obtained via [`PartialReflect::reflect_owned`].
///
/// ["kinds"]: ReflectKind
pub enum ReflectOwned {
    Struct(Box<dyn Struct>),
    TupleStruct(Box<dyn TupleStruct>),
    Tuple(Box<dyn Tuple>),
    List(Box<dyn List>),
    Array(Box<dyn Array>),
    Map(Box<dyn Map>),
    Set(Box<dyn Set>),
    Enum(Box<dyn Enum>),
    #[cfg(feature = "functions")]
    Function(Box<dyn Function>),
    Value(Box<dyn PartialReflect>),
}
impl_reflect_kind_conversions!(ReflectOwned);

impl ReflectOwned {
    impl_cast_method!(into_struct: Struct => Box<dyn Struct>);
    impl_cast_method!(into_tuple_struct: TupleStruct => Box<dyn TupleStruct>);
    impl_cast_method!(into_tuple: Tuple => Box<dyn Tuple>);
    impl_cast_method!(into_list: List => Box<dyn List>);
    impl_cast_method!(into_array: Array => Box<dyn Array>);
    impl_cast_method!(into_map: Map => Box<dyn Map>);
    impl_cast_method!(into_set: Set => Box<dyn Set>);
    impl_cast_method!(into_enum: Enum => Box<dyn Enum>);
    impl_cast_method!(into_value: Value => Box<dyn PartialReflect>);
}

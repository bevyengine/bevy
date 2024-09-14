#[cfg(feature = "functions")]
use crate::func::Function;
use crate::{Array, Enum, List, Map, PartialReflect, Set, Struct, Tuple, TupleStruct};

/// A zero-sized enumeration of the "kinds" of a reflected type.
///
/// A [`ReflectKind`] is obtained via [`PartialReflect::reflect_kind`],
/// or via [`ReflectRef::kind`],[`ReflectMut::kind`] or [`ReflectOwned::kind`].
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ReflectKind {
    Struct,
    TupleStruct,
    Tuple,
    List,
    Array,
    Map,
    Set,
    Enum,
    #[cfg(feature = "functions")]
    Function,
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

macro_rules! impl_reflect_enum {
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

/// An immutable enumeration of "kinds" of a reflected type.
///
/// Each variant contains a trait object with methods specific to a kind of
/// type.
///
/// A [`ReflectRef`] is obtained via [`PartialReflect::reflect_ref`].
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
impl_reflect_enum!(ReflectRef<'_>);

/// A mutable enumeration of "kinds" of a reflected type.
///
/// Each variant contains a trait object with methods specific to a kind of
/// type.
///
/// A [`ReflectMut`] is obtained via [`PartialReflect::reflect_mut`].
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
impl_reflect_enum!(ReflectMut<'_>);

/// An owned enumeration of "kinds" of a reflected type.
///
/// Each variant contains a trait object with methods specific to a kind of
/// type.
///
/// A [`ReflectOwned`] is obtained via [`PartialReflect::reflect_owned`].
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
impl_reflect_enum!(ReflectOwned);

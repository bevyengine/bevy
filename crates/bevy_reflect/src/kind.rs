use alloc::boxed::Box;
use thiserror::Error;

#[cfg(feature = "functions")]
use crate::func::Function;
use crate::{Array, Enum, List, Map, PartialReflect, Set, Struct, Tuple, TupleStruct};

/// An enumeration of the "kinds" of a reflected type.
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
    /// An opaque type.
    ///
    /// This most often represents a type where it is either impossible, difficult,
    /// or unuseful to reflect the type further.
    ///
    /// This includes types like `String` and `Instant`.
    ///
    /// Despite not technically being opaque types,
    /// primitives like `u32` `i32` are considered opaque for the purposes of reflection.
    ///
    /// Additionally, any type that [derives `Reflect`] with the `#[reflect(opaque)]` attribute
    /// will be considered an opaque type.
    ///
    /// [derives `Reflect`]: bevy_reflect_derive::Reflect
    Opaque,
}

impl core::fmt::Display for ReflectKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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
            ReflectKind::Opaque => f.pad("opaque"),
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
                    Self::Opaque(_) => ReflectKind::Opaque,
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
                    $name::Opaque(_) => Self::Opaque,
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
    /// Expected kind.
    pub expected: ReflectKind,
    /// Received kind.
    pub received: ReflectKind,
}

macro_rules! impl_cast_method {
    ($name:ident : Opaque => $retval:ty) => {
        #[doc = "Attempts a cast to a [`PartialReflect`] trait object."]
        #[doc = "\n\nReturns an error if `self` is not the [`Self::Opaque`] variant."]
        pub fn $name(self) -> Result<$retval, ReflectKindMismatchError> {
            match self {
                Self::Opaque(value) => Ok(value),
                _ => Err(ReflectKindMismatchError {
                    expected: ReflectKind::Opaque,
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
    /// An immutable reference to a [struct-like] type.
    ///
    /// [struct-like]: Struct
    Struct(&'a dyn Struct),
    /// An immutable reference to a [tuple-struct-like] type.
    ///
    /// [tuple-struct-like]: TupleStruct
    TupleStruct(&'a dyn TupleStruct),
    /// An immutable reference to a [tuple-like] type.
    ///
    /// [tuple-like]: Tuple
    Tuple(&'a dyn Tuple),
    /// An immutable reference to a [list-like] type.
    ///
    /// [list-like]: List
    List(&'a dyn List),
    /// An immutable reference to an [array-like] type.
    ///
    /// [array-like]: Array
    Array(&'a dyn Array),
    /// An immutable reference to a [map-like] type.
    ///
    /// [map-like]: Map
    Map(&'a dyn Map),
    /// An immutable reference to a [set-like] type.
    ///
    /// [set-like]: Set
    Set(&'a dyn Set),
    /// An immutable reference to an [enum-like] type.
    ///
    /// [enum-like]: Enum
    Enum(&'a dyn Enum),
    /// An immutable reference to a [function-like] type.
    ///
    /// [function-like]: Function
    #[cfg(feature = "functions")]
    Function(&'a dyn Function),
    /// An immutable reference to an [opaque] type.
    ///
    /// [opaque]: ReflectKind::Opaque
    Opaque(&'a dyn PartialReflect),
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
    impl_cast_method!(as_opaque: Opaque => &'a dyn PartialReflect);
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
    /// A mutable reference to a [struct-like] type.
    ///
    /// [struct-like]: Struct
    Struct(&'a mut dyn Struct),
    /// A mutable reference to a [tuple-struct-like] type.
    ///
    /// [tuple-struct-like]: TupleStruct
    TupleStruct(&'a mut dyn TupleStruct),
    /// A mutable reference to a [tuple-like] type.
    ///
    /// [tuple-like]: Tuple
    Tuple(&'a mut dyn Tuple),
    /// A mutable reference to a [list-like] type.
    ///
    /// [list-like]: List
    List(&'a mut dyn List),
    /// A mutable reference to an [array-like] type.
    ///
    /// [array-like]: Array
    Array(&'a mut dyn Array),
    /// A mutable reference to a [map-like] type.
    ///
    /// [map-like]: Map
    Map(&'a mut dyn Map),
    /// A mutable reference to a [set-like] type.
    ///
    /// [set-like]: Set
    Set(&'a mut dyn Set),
    /// A mutable reference to an [enum-like] type.
    ///
    /// [enum-like]: Enum
    Enum(&'a mut dyn Enum),
    #[cfg(feature = "functions")]
    /// A mutable reference to a [function-like] type.
    ///
    /// [function-like]: Function
    Function(&'a mut dyn Function),
    /// A mutable reference to an [opaque] type.
    ///
    /// [opaque]: ReflectKind::Opaque
    Opaque(&'a mut dyn PartialReflect),
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
    impl_cast_method!(as_opaque: Opaque => &'a mut dyn PartialReflect);
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
    /// An owned [struct-like] type.
    ///
    /// [struct-like]: Struct
    Struct(Box<dyn Struct>),
    /// An owned [tuple-struct-like] type.
    ///
    /// [tuple-struct-like]: TupleStruct
    TupleStruct(Box<dyn TupleStruct>),
    /// An owned [tuple-like] type.
    ///
    /// [tuple-like]: Tuple
    Tuple(Box<dyn Tuple>),
    /// An owned [list-like] type.
    ///
    /// [list-like]: List
    List(Box<dyn List>),
    /// An owned [array-like] type.
    ///
    /// [array-like]: Array
    Array(Box<dyn Array>),
    /// An owned [map-like] type.
    ///
    /// [map-like]: Map
    Map(Box<dyn Map>),
    /// An owned [set-like] type.
    ///
    /// [set-like]: Set
    Set(Box<dyn Set>),
    /// An owned [enum-like] type.
    ///
    /// [enum-like]: Enum
    Enum(Box<dyn Enum>),
    /// An owned [function-like] type.
    ///
    /// [function-like]: Function
    #[cfg(feature = "functions")]
    Function(Box<dyn Function>),
    /// An owned [opaque] type.
    ///
    /// [opaque]: ReflectKind::Opaque
    Opaque(Box<dyn PartialReflect>),
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
    impl_cast_method!(into_value: Opaque => Box<dyn PartialReflect>);
}

#[cfg(test)]
mod tests {
    use alloc::vec;
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn should_cast_ref() {
        let value = vec![1, 2, 3];

        let result = value.reflect_ref().as_list();
        assert!(result.is_ok());

        let result = value.reflect_ref().as_array();
        assert!(matches!(
            result,
            Err(ReflectKindMismatchError {
                expected: ReflectKind::Array,
                received: ReflectKind::List
            })
        ));
    }

    #[test]
    fn should_cast_mut() {
        let mut value: HashSet<i32> = HashSet::default();

        let result = value.reflect_mut().as_set();
        assert!(result.is_ok());

        let result = value.reflect_mut().as_map();
        assert!(matches!(
            result,
            Err(ReflectKindMismatchError {
                expected: ReflectKind::Map,
                received: ReflectKind::Set
            })
        ));
    }

    #[test]
    fn should_cast_owned() {
        let value = Box::new(Some(123));

        let result = value.reflect_owned().into_enum();
        assert!(result.is_ok());

        let value = Box::new(Some(123));

        let result = value.reflect_owned().into_struct();
        assert!(matches!(
            result,
            Err(ReflectKindMismatchError {
                expected: ReflectKind::Struct,
                received: ReflectKind::Enum
            })
        ));
    }
}

use std::fmt;

use super::{AccessError, ReflectPathError};
use crate::{Reflect, ReflectMut, ReflectRef, VariantType};
use thiserror::Error;

type InnerResult<T> = Result<Option<T>, Error<'static>>;

#[derive(Debug, PartialEq, Eq, Error)]
pub(super) enum Error<'a> {
    #[error(
        "the current {ty} doesn't have the {} {}",
        access.kind(),
        access.display_value(),
    )]
    Access {
        ty: TypeShape,
        access: AccessRef<'a>,
    },

    #[error("invalid type shape: expected {expected} but found a reflect {actual}")]
    Type {
        expected: TypeShape,
        actual: TypeShape,
    },

    #[error("invalid enum access: expected {expected} variant but found {actual} variant")]
    Enum {
        expected: TypeShape,
        actual: TypeShape,
    },
}

impl<'a> Error<'a> {
    fn with_offset(self, offset: usize) -> ReflectPathError<'a> {
        let error = AccessError(self);
        ReflectPathError::InvalidAccess { offset, error }
    }
}
impl Error<'static> {
    fn bad_enum_variant(expected: TypeShape, actual: impl Into<TypeShape>) -> Self {
        let actual = actual.into();
        Error::Enum { expected, actual }
    }
    fn bad_type(expected: TypeShape, actual: impl Into<TypeShape>) -> Self {
        let actual = actual.into();
        Error::Type { expected, actual }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(super) enum TypeShape {
    Struct,
    TupleStruct,
    Tuple,
    List,
    Array,
    Map,
    Enum,
    Value,
    Unit,
}

impl fmt::Display for TypeShape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            TypeShape::Struct => "struct",
            TypeShape::TupleStruct => "tuple struct",
            TypeShape::Tuple => "tuple",
            TypeShape::List => "list",
            TypeShape::Array => "array",
            TypeShape::Map => "map",
            TypeShape::Enum => "enum",
            TypeShape::Value => "value",
            TypeShape::Unit => "unit",
        };
        write!(f, "{name}")
    }
}
impl<'a> From<ReflectRef<'a>> for TypeShape {
    fn from(value: ReflectRef<'a>) -> Self {
        match value {
            ReflectRef::Struct(_) => TypeShape::Struct,
            ReflectRef::TupleStruct(_) => TypeShape::TupleStruct,
            ReflectRef::Tuple(_) => TypeShape::Tuple,
            ReflectRef::List(_) => TypeShape::List,
            ReflectRef::Array(_) => TypeShape::Array,
            ReflectRef::Map(_) => TypeShape::Map,
            ReflectRef::Enum(_) => TypeShape::Enum,
            ReflectRef::Value(_) => TypeShape::Value,
        }
    }
}
impl From<VariantType> for TypeShape {
    fn from(value: VariantType) -> Self {
        match value {
            VariantType::Struct => TypeShape::Struct,
            VariantType::Tuple => TypeShape::Tuple,
            VariantType::Unit => TypeShape::Unit,
        }
    }
}

/// A singular owned element access within a path.
///
/// Can be applied to a `dyn Reflect` to get a reference to the targeted element.
///
/// A path is composed of multiple accesses in sequence.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(super) enum Access {
    Field(Box<str>),
    FieldIndex(usize),
    TupleIndex(usize),
    ListIndex(usize),
}

impl fmt::Display for Access {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Access::Field(field) => write!(f, ".{field}"),
            Access::FieldIndex(index) => write!(f, "#{index}"),
            Access::TupleIndex(index) => write!(f, ".{index}"),
            Access::ListIndex(index) => write!(f, "[{index}]"),
        }
    }
}

impl Access {
    pub(super) fn as_ref(&self) -> AccessRef<'_> {
        match self {
            Self::Field(value) => AccessRef::Field(value),
            Self::FieldIndex(value) => AccessRef::FieldIndex(*value),
            Self::TupleIndex(value) => AccessRef::TupleIndex(*value),
            Self::ListIndex(value) => AccessRef::ListIndex(*value),
        }
    }
}

/// A singular borrowed element access within a path.
///
/// Can be applied to a `dyn Reflect` to get a reference to the targeted element.
///
/// Does not own the backing store it's sourced from.
/// For an owned version, you can convert one to an [`Access`] with [`AccessRef::to_owned`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AccessRef<'a> {
    Field(&'a str),
    FieldIndex(usize),
    TupleIndex(usize),
    ListIndex(usize),
}

impl<'a> AccessRef<'a> {
    pub(super) fn to_owned(self) -> Access {
        match self {
            Self::Field(value) => Access::Field(value.to_string().into_boxed_str()),
            Self::FieldIndex(value) => Access::FieldIndex(value),
            Self::TupleIndex(value) => Access::TupleIndex(value),
            Self::ListIndex(value) => Access::ListIndex(value),
        }
    }

    fn display_value(&self) -> &dyn fmt::Display {
        match self {
            Self::Field(value) => value,
            Self::FieldIndex(value) | Self::TupleIndex(value) | Self::ListIndex(value) => value,
        }
    }
    fn kind(&self) -> &'static str {
        match self {
            Self::Field(_) => "field",
            Self::FieldIndex(_) => "field index",
            Self::TupleIndex(_) | Self::ListIndex(_) => "index",
        }
    }

    pub(super) fn element(
        self,
        base: &dyn Reflect,
        offset: usize,
    ) -> Result<&dyn Reflect, ReflectPathError<'a>> {
        let ty = base.reflect_ref().into();
        self.element_inner(base)
            .and_then(|maybe| maybe.ok_or(Error::Access { ty, access: self }))
            .map_err(|err| err.with_offset(offset))
    }

    fn element_inner(self, base: &dyn Reflect) -> InnerResult<&dyn Reflect> {
        use ReflectRef::*;
        match (self, base.reflect_ref()) {
            (Self::Field(field), Struct(struct_ref)) => Ok(struct_ref.field(field)),
            (Self::Field(field), Enum(enum_ref)) => match enum_ref.variant_type() {
                VariantType::Struct => Ok(enum_ref.field(field)),
                actual => Err(Error::bad_enum_variant(TypeShape::Struct, actual)),
            },
            (Self::FieldIndex(index), Struct(struct_ref)) => Ok(struct_ref.field_at(index)),
            (Self::FieldIndex(index), Enum(enum_ref)) => match enum_ref.variant_type() {
                VariantType::Struct => Ok(enum_ref.field_at(index)),
                actual => Err(Error::bad_enum_variant(TypeShape::Struct, actual)),
            },
            (Self::TupleIndex(index), TupleStruct(tuple)) => Ok(tuple.field(index)),
            (Self::TupleIndex(index), Tuple(tuple)) => Ok(tuple.field(index)),
            (Self::TupleIndex(index), Enum(enum_ref)) => match enum_ref.variant_type() {
                VariantType::Tuple => Ok(enum_ref.field_at(index)),
                actual => Err(Error::bad_enum_variant(TypeShape::Tuple, actual)),
            },
            (Self::ListIndex(index), List(list)) => Ok(list.get(index)),
            (Self::ListIndex(index), Array(list)) => Ok(list.get(index)),
            (Self::ListIndex(_), actual) => Err(Error::bad_type(TypeShape::List, actual)),
            (_, actual) => Err(Error::bad_type(TypeShape::Struct, actual)),
        }
    }

    pub(super) fn element_mut(
        self,
        base: &mut dyn Reflect,
        offset: usize,
    ) -> Result<&mut dyn Reflect, ReflectPathError<'a>> {
        let ty = base.reflect_ref().into();
        self.element_inner_mut(base)
            .and_then(|maybe| maybe.ok_or(Error::Access { ty, access: self }))
            .map_err(|err| err.with_offset(offset))
    }

    fn element_inner_mut(self, base: &mut dyn Reflect) -> InnerResult<&mut dyn Reflect> {
        use ReflectMut::*;
        let base_kind: TypeShape = base.reflect_ref().into();
        match (self, base.reflect_mut()) {
            (Self::Field(field), Struct(struct_mut)) => Ok(struct_mut.field_mut(field)),
            (Self::Field(field), Enum(enum_mut)) => match enum_mut.variant_type() {
                VariantType::Struct => Ok(enum_mut.field_mut(field)),
                actual => Err(Error::bad_enum_variant(TypeShape::Struct, actual)),
            },
            (Self::FieldIndex(index), Struct(struct_mut)) => Ok(struct_mut.field_at_mut(index)),
            (Self::FieldIndex(index), Enum(enum_mut)) => match enum_mut.variant_type() {
                VariantType::Struct => Ok(enum_mut.field_at_mut(index)),
                actual => Err(Error::bad_enum_variant(TypeShape::Struct, actual)),
            },
            (Self::TupleIndex(index), TupleStruct(tuple)) => Ok(tuple.field_mut(index)),
            (Self::TupleIndex(index), Tuple(tuple)) => Ok(tuple.field_mut(index)),
            (Self::TupleIndex(index), Enum(enum_mut)) => match enum_mut.variant_type() {
                VariantType::Tuple => Ok(enum_mut.field_at_mut(index)),
                actual => Err(Error::bad_enum_variant(TypeShape::Tuple, actual)),
            },
            (Self::ListIndex(index), List(list)) => Ok(list.get_mut(index)),
            (Self::ListIndex(index), Array(list)) => Ok(list.get_mut(index)),
            (Self::ListIndex(_), _) => Err(Error::bad_type(TypeShape::List, base_kind)),
            (_, _) => Err(Error::bad_type(TypeShape::Struct, base_kind)),
        }
    }
}

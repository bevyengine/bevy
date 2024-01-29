use std::fmt;

use thiserror::Error;

use super::{Access, Offset};
use crate::{ReflectMut, ReflectRef, VariantType};

#[derive(Debug, PartialEq, Eq, Error, Clone, Copy)]
pub(super) enum AccessErrorKind {
    #[error("Invalid access for {0} type")]
    MissingAccess(TypeKind),

    #[error("Invalid type kind, expected {expected} type, got {actual} type")]
    InvalidType {
        expected: TypeKind,
        actual: TypeKind,
    },
    #[error("Invalid variant kind, expected {expected:?} variant, got {actual:?} variant")]
    InvalidEnumVariant {
        expected: VariantType,
        actual: VariantType,
    },
}

impl AccessErrorKind {
    pub(super) fn with_access<'a>(
        self,
        access: &Access<'a>,
        offset: Offset,
    ) -> crate::ReflectPathError<'a> {
        crate::ReflectPathError::InvalidAccess(AccessError::new(self, access.clone(), offset))
    }

    pub(super) fn invalid_type(expected: impl Into<TypeKind>, actual: impl Into<TypeKind>) -> Self {
        Self::InvalidType {
            expected: expected.into(),
            actual: actual.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AccessOffsetDisplay(Offset);

impl fmt::Display for AccessOffsetDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(index) = self.0.get() {
            write!(f, " at offset {index} in path")?;
        }
        Ok(())
    }
}

/// An error originating from an [`Access`] of an element within a type.
///
/// Use the `Display` impl of this type to get informations on the error.
/// Some sample messages:
/// ```text
/// Error accessing element at offset 10 in path with '.x': Invalid access for tuple type.
/// Error accessing element with '[0]': Invalid type kind, expected list type, got struct type.
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("Error accessing element{offset} with '{access}': {error}.")]
pub struct AccessError<'a> {
    error: AccessErrorKind,
    access: Access<'a>,
    offset: AccessOffsetDisplay,
}

impl<'a> AccessError<'a> {
    pub(super) fn new(error: AccessErrorKind, access: Access<'a>, offset: Offset) -> Self {
        Self {
            error,
            access,
            offset: AccessOffsetDisplay(offset),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(super) enum TypeKind {
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

impl fmt::Display for TypeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            TypeKind::Struct => "struct",
            TypeKind::TupleStruct => "tuple struct",
            TypeKind::Tuple => "tuple",
            TypeKind::List => "list",
            TypeKind::Array => "array",
            TypeKind::Map => "map",
            TypeKind::Enum => "enum",
            TypeKind::Value => "value",
            TypeKind::Unit => "unit",
        };
        write!(f, "{name}")
    }
}
impl<'a> From<ReflectRef<'a>> for TypeKind {
    fn from(value: ReflectRef<'a>) -> Self {
        match value {
            ReflectRef::Struct(_) => TypeKind::Struct,
            ReflectRef::TupleStruct(_) => TypeKind::TupleStruct,
            ReflectRef::Tuple(_) => TypeKind::Tuple,
            ReflectRef::List(_) => TypeKind::List,
            ReflectRef::Array(_) => TypeKind::Array,
            ReflectRef::Map(_) => TypeKind::Map,
            ReflectRef::Enum(_) => TypeKind::Enum,
            ReflectRef::Value(_) => TypeKind::Value,
        }
    }
}
impl<'a> From<ReflectMut<'a>> for TypeKind {
    fn from(value: ReflectMut<'a>) -> Self {
        match value {
            ReflectMut::Struct(_) => TypeKind::Struct,
            ReflectMut::TupleStruct(_) => TypeKind::TupleStruct,
            ReflectMut::Tuple(_) => TypeKind::Tuple,
            ReflectMut::List(_) => TypeKind::List,
            ReflectMut::Array(_) => TypeKind::Array,
            ReflectMut::Map(_) => TypeKind::Map,
            ReflectMut::Enum(_) => TypeKind::Enum,
            ReflectMut::Value(_) => TypeKind::Value,
        }
    }
}
impl From<VariantType> for TypeKind {
    fn from(value: VariantType) -> Self {
        match value {
            VariantType::Struct => TypeKind::Struct,
            VariantType::Tuple => TypeKind::Tuple,
            VariantType::Unit => TypeKind::Unit,
        }
    }
}

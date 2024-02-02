use std::fmt;

use super::Access;
use crate::{Reflect, ReflectMut, ReflectRef, VariantType};

/// The kind of [`AccessError`], along with some kind-specific information.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AccessErrorKind {
    /// An error that occurs when a certain type doesn't
    /// contain the value referenced by the [`Access`].
    MissingField(TypeKind),

    /// An error that occurs when using an [`Access`] on the wrong type.
    /// (i.e. a [`ListIndex`](Access::ListIndex) on a struct, or a [`TupleIndex`](Access::TupleIndex) on a list)
    IncompatibleTypes {
        /// The [`TypeKind`] that was expected based on the [`Access`].
        expected: TypeKind,
        /// The actual [`TypeKind`] that was found.
        actual: TypeKind,
    },

    /// An error that occurs when using an [`Access`] on the wrong enum variant.
    /// (i.e. a [`ListIndex`](Access::ListIndex) on a struct variant, or a [`TupleIndex`](Access::TupleIndex) on a unit variant)
    IncompatibleEnumVariantTypes {
        /// The [`VariantType`] that was expected based on the [`Access`].
        expected: VariantType,
        /// The actual [`VariantType`] that was found.
        actual: VariantType,
    },
}

impl AccessErrorKind {
    pub(super) fn with_access(self, access: Access, offset: Option<usize>) -> AccessError {
        AccessError {
            kind: self,
            access,
            offset,
        }
    }
}

/// An error originating from an [`Access`] of an element within a type.
///
/// Use the `Display` impl of this type to get information on the error.
///
/// Some sample messages:
///
/// ```text
/// Error accessing element with `.alpha` access (offset 14): The struct accessed doesn't have an "alpha" field
/// Error accessing element with '[0]' access: Expected index access to access a list, found a struct instead.
/// Error accessing element with '.4' access: Expected variant index access to access a Tuple variant, found a Unit variant instead.
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessError<'a> {
    pub(super) kind: AccessErrorKind,
    pub(super) access: Access<'a>,
    pub(super) offset: Option<usize>,
}

impl<'a> AccessError<'a> {
    /// Returns the kind of [`AccessError`].
    pub const fn kind(&self) -> &AccessErrorKind {
        &self.kind
    }

    /// The returns the [`Access`] that this [`AccessError`] occured in.
    pub const fn access(&self) -> &Access {
        &self.access
    }

    /// If the [`Access`] was created with a parser or an offset was manually provided,
    /// returns the offset of the [`Access`] in it's path string.
    pub const fn offset(&self) -> Option<&usize> {
        self.offset.as_ref()
    }
}
impl std::fmt::Display for AccessError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let AccessError {
            kind,
            access,
            offset,
        } = self;

        write!(f, "Error accessing element with `{access}` access")?;
        if let Some(offset) = offset {
            write!(f, "(offset {offset})")?;
        }
        write!(f, ": ")?;

        match kind {
            AccessErrorKind::MissingField(type_accessed) => {
                match access {
                    Access::Field(field) => write!(
                        f,
                        "The {type_accessed} accessed doesn't have {} `{}` field",
                        if let Some("a" | "e" | "i" | "o" | "u") = field.get(0..1) {
                            "an"
                        } else {
                            "a"
                        },
                        access.display_value()
                    ),
                    Access::FieldIndex(_) => write!(
                        f,
                        "The {type_accessed} accessed doesn't have field index `{}`",
                        access.display_value(),
                    ),
                    Access::TupleIndex(_) | Access::ListIndex(_) => write!(
                        f,
                        "The {type_accessed} accessed doesn't have index `{}`",
                        access.display_value()
                    )
                }
            }
            AccessErrorKind::IncompatibleTypes { expected, actual } => write!(
                f,
                "Expected {} access to access a {expected}, found a {actual} instead.",
                access.kind()
            ),
            AccessErrorKind::IncompatibleEnumVariantTypes { expected, actual } => write!(
                f,
                "Expected variant {} access to access a {expected:?} variant, found a {actual:?} variant instead.",
                access.kind()
            ),
        }
    }
}
impl std::error::Error for AccessError<'_> {}

/// The kind of the type trying to be accessed.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[allow(missing_docs /* Variants are self-explanatory */)]
pub enum TypeKind {
    Struct,
    TupleStruct,
    Tuple,
    FixedLenList,
    List,
    Array,
    Map,
    Enum,
    Value,
    Unit,
}

impl fmt::Display for TypeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeKind::Struct => f.pad("struct"),
            TypeKind::TupleStruct => f.pad("tuple struct"),
            TypeKind::Tuple => f.pad("tuple"),
            TypeKind::FixedLenList => f.pad("fixed len list"),
            TypeKind::List => f.pad("list"),
            TypeKind::Array => f.pad("array"),
            TypeKind::Map => f.pad("map"),
            TypeKind::Enum => f.pad("enum"),
            TypeKind::Value => f.pad("value"),
            TypeKind::Unit => f.pad("unit"),
        }
    }
}
impl From<ReflectRef<'_>> for TypeKind {
    fn from(value: ReflectRef) -> Self {
        match value {
            ReflectRef::Struct(_) => TypeKind::Struct,
            ReflectRef::TupleStruct(_) => TypeKind::TupleStruct,
            ReflectRef::Tuple(_) => TypeKind::Tuple,
            ReflectRef::FixedLenList(_) => TypeKind::FixedLenList,
            ReflectRef::List(_) => TypeKind::List,
            ReflectRef::Array(_) => TypeKind::Array,
            ReflectRef::Map(_) => TypeKind::Map,
            ReflectRef::Enum(_) => TypeKind::Enum,
            ReflectRef::Value(_) => TypeKind::Value,
        }
    }
}
impl From<&dyn Reflect> for TypeKind {
    fn from(value: &dyn Reflect) -> Self {
        value.reflect_ref().into()
    }
}
impl From<ReflectMut<'_>> for TypeKind {
    fn from(value: ReflectMut) -> Self {
        match value {
            ReflectMut::Struct(_) => TypeKind::Struct,
            ReflectMut::TupleStruct(_) => TypeKind::TupleStruct,
            ReflectMut::Tuple(_) => TypeKind::Tuple,
            ReflectMut::FixedLenList(_) => TypeKind::FixedLenList,
            ReflectMut::List(_) => TypeKind::List,
            ReflectMut::Array(_) => TypeKind::Array,
            ReflectMut::Map(_) => TypeKind::Map,
            ReflectMut::Enum(_) => TypeKind::Enum,
            ReflectMut::Value(_) => TypeKind::Value,
        }
    }
}
impl From<&mut dyn Reflect> for TypeKind {
    fn from(value: &mut dyn Reflect) -> Self {
        value.reflect_ref().into()
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

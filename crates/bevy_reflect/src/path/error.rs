use std::fmt;

use super::Access;
use crate::{ReflectKind, VariantType};

/// The kind of [`AccessError`], along with some kind-specific information.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AccessErrorKind {
    /// An error that occurs when a certain type doesn't
    /// contain the value referenced by the [`Access`].
    MissingField(ReflectKind),

    /// An error that occurs when using an [`Access`] on the wrong type.
    /// (i.e. a [`ListIndex`](Access::ListIndex) on a struct, or a [`TupleIndex`](Access::TupleIndex) on a list)
    IncompatibleTypes {
        /// The [`ReflectKind`] that was expected based on the [`Access`].
        expected: ReflectKind,
        /// The actual [`ReflectKind`] that was found.
        actual: ReflectKind,
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

    /// The returns the [`Access`] that this [`AccessError`] occurred in.
    pub const fn access(&self) -> &Access {
        &self.access
    }

    /// If the [`Access`] was created with a parser or an offset was manually provided,
    /// returns the offset of the [`Access`] in its path string.
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

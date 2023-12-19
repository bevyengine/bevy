//! Representation for individual element accesses within a path.

use std::{borrow::Cow, fmt};

use super::ReflectPathError;
use crate::{Reflect, ReflectMut, ReflectRef, VariantType};
use thiserror::Error;

type InnerResult<'a, T> = Result<Option<T>, Error<'a>>;

/// An error originating from an [`Access`] of a reflected type.
#[derive(Debug, PartialEq, Eq, Error)]
pub enum Error<'a> {
    /// An error that occurs when a certain type doesn't
    /// contain the value contained in the [`Access`].
    #[error(
        "the current {ty} doesn't have the {} {}",
        access.kind(),
        access.display_value(),
    )]
    MissingAccess {
        /// The type of the object being accessed.
        ty: TypeKind,
        /// The [`Access`] used on the object.
        access: Access<'a>,
    },

    /// An error that occurs when using an [`Access`] on the wrong type.
    /// (i.e. a [`ListIndex`](Access::ListIndex) on a struct, or a [`TupleIndex`](Access::TupleIndex) on a list)
    #[error(
        "invalid type: expected {} access to use a type of {expected} but found a reflect {actual}",
        access.kind()
    )]
    InvalidType {
        /// The [`TypeKind`] that was expected based on the [`Access`].
        expected: TypeKind,
        /// The actual [`TypeKind`] that was found.
        actual: TypeKind,
        /// The [`Access`] used.
        access: Access<'a>,
    },

    /// An error that occurs when using an [`Access`] on the wrong enum variant.
    /// (i.e. a [`ListIndex`](Access::ListIndex) on a struct variant, or a [`TupleIndex`](Access::TupleIndex) on a unit variant)
    #[error(
        "invalid enum variant: expected {} access to use a type of {expected:?} variant but found {actual:?} variant",
        access.kind()
    )]
    InvalidEnumVariant {
        /// The [`VariantType`] that was expected based on the [`Access`].
        expected: VariantType,
        /// The actual [`VariantType`] that was found.
        actual: VariantType,
        /// The [`Access`] used.
        access: Access<'a>,
    },
}

impl<'a> Error<'a> {
    fn with_offset(self, offset: Option<usize>) -> ReflectPathError<'a> {
        ReflectPathError::InvalidAccess {
            offset,
            error: self,
        }
    }
}

/// The kind of the type trying to be accessed.
#[allow(missing_docs /* Variants are self-explanatory */)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TypeKind {
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

/// A singular element access within a path.
///
/// Can be applied to a [`dyn Reflect`](Reflect) to get a reference to the targeted element.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Access<'a> {
    /// A name-based field access on a struct.
    Field(Cow<'a, str>),
    /// A index-based field access on a struct.
    FieldIndex(usize),
    /// An index-based access on a tuple.
    TupleIndex(usize),
    /// An index-based access on a list.
    ListIndex(usize),
}

impl fmt::Display for Access<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Access::Field(field) => write!(f, ".{field}"),
            Access::FieldIndex(index) => write!(f, "#{index}"),
            Access::TupleIndex(index) => write!(f, ".{index}"),
            Access::ListIndex(index) => write!(f, "[{index}]"),
        }
    }
}

impl<'a> Access<'a> {
    /// If the [`Access`] is of variant [`Field`](Access::Field),
    /// the field's [`Cow<str>`] will be converted to it's owned
    /// counterpart, which doesn't require a lifetime.
    ///
    /// Otherwise does nothing.
    pub fn as_static(self) -> Access<'static> {
        match self {
            Self::Field(value) => Access::Field(Cow::Owned(value.to_string())),
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

    pub(super) fn element<'r>(
        &self,
        base: &'r dyn Reflect,
        offset: Option<usize>,
    ) -> Result<&'r dyn Reflect, ReflectPathError<'a>> {
        let ty = base.reflect_ref().into();
        self.element_inner(base)
            .and_then(|maybe| {
                maybe.ok_or(Error::MissingAccess {
                    ty,
                    access: self.clone(),
                })
            })
            .map_err(|err| err.with_offset(offset))
    }

    fn element_inner<'r>(&self, base: &'r dyn Reflect) -> InnerResult<'a, &'r dyn Reflect> {
        use ReflectRef::*;

        match (self, base.reflect_ref()) {
            (Self::Field(field), Struct(struct_ref)) => Ok(struct_ref.field(field.as_ref())),
            (Self::Field(field), Enum(enum_ref)) => match enum_ref.variant_type() {
                VariantType::Struct => Ok(enum_ref.field(field.as_ref())),
                actual => Err(Error::InvalidEnumVariant {
                    expected: VariantType::Struct,
                    actual,
                    access: self.clone(),
                }),
            },
            (&Self::FieldIndex(index), Struct(struct_ref)) => Ok(struct_ref.field_at(index)),
            (&Self::FieldIndex(index), Enum(enum_ref)) => match enum_ref.variant_type() {
                VariantType::Struct => Ok(enum_ref.field_at(index)),
                actual => Err(Error::InvalidEnumVariant {
                    expected: VariantType::Struct,
                    actual,
                    access: self.clone(),
                }),
            },
            (&Self::TupleIndex(index), TupleStruct(tuple)) => Ok(tuple.field(index)),
            (&Self::TupleIndex(index), Tuple(tuple)) => Ok(tuple.field(index)),
            (&Self::TupleIndex(index), Enum(enum_ref)) => match enum_ref.variant_type() {
                VariantType::Tuple => Ok(enum_ref.field_at(index)),
                actual => Err(Error::InvalidEnumVariant {
                    expected: VariantType::Tuple,
                    actual,
                    access: self.clone(),
                }),
            },
            (&Self::ListIndex(index), List(list)) => Ok(list.get(index)),
            (&Self::ListIndex(index), Array(list)) => Ok(list.get(index)),
            (&Self::ListIndex(_), actual) => Err(Error::InvalidType {
                expected: TypeKind::List,
                actual: actual.into(),
                access: self.clone(),
            }),
            (_, actual) => Err(Error::InvalidType {
                expected: TypeKind::Struct,
                actual: actual.into(),
                access: self.clone(),
            }),
        }
    }

    pub(super) fn element_mut<'r>(
        &self,
        base: &'r mut dyn Reflect,
        offset: Option<usize>,
    ) -> Result<&'r mut dyn Reflect, ReflectPathError<'a>> {
        let ty = base.reflect_ref().into();
        self.element_inner_mut(base)
            .and_then(|maybe| {
                maybe.ok_or(Error::MissingAccess {
                    ty,
                    access: self.clone(),
                })
            })
            .map_err(|err| err.with_offset(offset))
    }

    fn element_inner_mut<'r>(
        &self,
        base: &'r mut dyn Reflect,
    ) -> InnerResult<'a, &'r mut dyn Reflect> {
        use ReflectMut::*;

        match (self, base.reflect_mut()) {
            (Self::Field(field), Struct(struct_mut)) => Ok(struct_mut.field_mut(field.as_ref())),
            (Self::Field(field), Enum(enum_mut)) => match enum_mut.variant_type() {
                VariantType::Struct => Ok(enum_mut.field_mut(field.as_ref())),
                actual => Err(Error::InvalidEnumVariant {
                    expected: VariantType::Struct,
                    actual,
                    access: self.clone(),
                }),
            },
            (&Self::FieldIndex(index), Struct(struct_mut)) => Ok(struct_mut.field_at_mut(index)),
            (&Self::FieldIndex(index), Enum(enum_mut)) => match enum_mut.variant_type() {
                VariantType::Struct => Ok(enum_mut.field_at_mut(index)),
                actual => Err(Error::InvalidEnumVariant {
                    expected: VariantType::Struct,
                    actual,
                    access: self.clone(),
                }),
            },
            (&Self::TupleIndex(index), TupleStruct(tuple)) => Ok(tuple.field_mut(index)),
            (&Self::TupleIndex(index), Tuple(tuple)) => Ok(tuple.field_mut(index)),
            (&Self::TupleIndex(index), Enum(enum_mut)) => match enum_mut.variant_type() {
                VariantType::Tuple => Ok(enum_mut.field_at_mut(index)),
                actual => Err(Error::InvalidEnumVariant {
                    expected: VariantType::Tuple,
                    actual,
                    access: self.clone(),
                }),
            },
            (&Self::ListIndex(index), List(list)) => Ok(list.get_mut(index)),
            (&Self::ListIndex(index), Array(list)) => Ok(list.get_mut(index)),
            (&Self::ListIndex(_), actual) => Err(Error::InvalidType {
                expected: TypeKind::List,
                actual: actual.into(),
                access: self.clone(),
            }),
            (_, actual) => Err(Error::InvalidType {
                expected: TypeKind::Struct,
                actual: actual.into(),
                access: self.clone(),
            }),
        }
    }
}

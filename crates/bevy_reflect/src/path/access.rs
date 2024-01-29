//! Representation for individual element accesses within a path.

use std::{borrow::Cow, fmt};

use super::error::{AccessErrorKind, TypeKind};
use super::{Offset, ReflectPathError};
use crate::{Reflect, ReflectMut, ReflectRef, VariantType};

type InnerResult<T> = Result<Option<T>, AccessErrorKind>;

/// A singular element access within a path.
/// Multiple accesses can be combined into a [`ParsedPath`](super::ParsedPath).
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
    /// Converts this into an "owned" value.
    ///
    /// If the [`Access`] is of variant [`Field`](Access::Field),
    /// the field's [`Cow<str>`] will be converted to it's owned
    /// counterpart, which doesn't require a reference.
    pub fn into_owned(self) -> Access<'static> {
        match self {
            Self::Field(value) => Access::Field(Cow::Owned(value.into_owned())),
            Self::FieldIndex(value) => Access::FieldIndex(value),
            Self::TupleIndex(value) => Access::TupleIndex(value),
            Self::ListIndex(value) => Access::ListIndex(value),
        }
    }

    pub(super) fn element<'r>(
        &self,
        base: &'r dyn Reflect,
        offset: Offset,
    ) -> Result<&'r dyn Reflect, ReflectPathError<'a>> {
        let kind = base.reflect_ref().into();
        self.element_inner(base)
            .and_then(|maybe| maybe.ok_or(AccessErrorKind::MissingAccess(kind)))
            .map_err(|err| err.with_access(self, offset))
    }

    fn element_inner<'r>(&self, base: &'r dyn Reflect) -> InnerResult<&'r dyn Reflect> {
        use ReflectRef::*;

        let invalid_variant =
            |expected, actual| AccessErrorKind::InvalidEnumVariant { expected, actual };

        match (self, base.reflect_ref()) {
            (Self::Field(field), Struct(struct_ref)) => Ok(struct_ref.field(field.as_ref())),
            (Self::Field(field), Enum(enum_ref)) => match enum_ref.variant_type() {
                VariantType::Struct => Ok(enum_ref.field(field.as_ref())),
                actual => Err(invalid_variant(VariantType::Struct, actual)),
            },
            (&Self::FieldIndex(index), Struct(struct_ref)) => Ok(struct_ref.field_at(index)),
            (&Self::FieldIndex(index), Enum(enum_ref)) => match enum_ref.variant_type() {
                VariantType::Struct => Ok(enum_ref.field_at(index)),
                actual => Err(invalid_variant(VariantType::Struct, actual)),
            },
            (&Self::TupleIndex(index), TupleStruct(tuple)) => Ok(tuple.field(index)),
            (&Self::TupleIndex(index), Tuple(tuple)) => Ok(tuple.field(index)),
            (&Self::TupleIndex(index), Enum(enum_ref)) => match enum_ref.variant_type() {
                VariantType::Tuple => Ok(enum_ref.field_at(index)),
                actual => Err(invalid_variant(VariantType::Tuple, actual)),
            },
            (&Self::ListIndex(index), List(list)) => Ok(list.get(index)),
            (&Self::ListIndex(index), Array(list)) => Ok(list.get(index)),
            (&Self::ListIndex(_), actual) => {
                Err(AccessErrorKind::invalid_type(TypeKind::List, actual))
            }
            (_, actual) => Err(AccessErrorKind::invalid_type(TypeKind::Struct, actual)),
        }
    }

    pub(super) fn element_mut<'r>(
        &self,
        base: &'r mut dyn Reflect,
        offset: Offset,
    ) -> Result<&'r mut dyn Reflect, ReflectPathError<'a>> {
        let kind = base.reflect_ref().into();
        self.element_inner_mut(base)
            .and_then(|maybe| maybe.ok_or(AccessErrorKind::MissingAccess(kind)))
            .map_err(|err| err.with_access(self, offset))
    }

    fn element_inner_mut<'r>(&self, base: &'r mut dyn Reflect) -> InnerResult<&'r mut dyn Reflect> {
        use ReflectMut::*;

        let invalid_variant =
            |expected, actual| AccessErrorKind::InvalidEnumVariant { expected, actual };

        match (self, base.reflect_mut()) {
            (Self::Field(field), Struct(struct_mut)) => Ok(struct_mut.field_mut(field.as_ref())),
            (Self::Field(field), Enum(enum_mut)) => match enum_mut.variant_type() {
                VariantType::Struct => Ok(enum_mut.field_mut(field.as_ref())),
                actual => Err(invalid_variant(VariantType::Struct, actual)),
            },
            (&Self::FieldIndex(index), Struct(struct_mut)) => Ok(struct_mut.field_at_mut(index)),
            (&Self::FieldIndex(index), Enum(enum_mut)) => match enum_mut.variant_type() {
                VariantType::Struct => Ok(enum_mut.field_at_mut(index)),
                actual => Err(invalid_variant(VariantType::Struct, actual)),
            },
            (&Self::TupleIndex(index), TupleStruct(tuple)) => Ok(tuple.field_mut(index)),
            (&Self::TupleIndex(index), Tuple(tuple)) => Ok(tuple.field_mut(index)),
            (&Self::TupleIndex(index), Enum(enum_mut)) => match enum_mut.variant_type() {
                VariantType::Tuple => Ok(enum_mut.field_at_mut(index)),
                actual => Err(invalid_variant(VariantType::Tuple, actual)),
            },
            (&Self::ListIndex(index), List(list)) => Ok(list.get_mut(index)),
            (&Self::ListIndex(index), Array(list)) => Ok(list.get_mut(index)),
            (&Self::ListIndex(_), actual) => {
                Err(AccessErrorKind::invalid_type(TypeKind::List, actual))
            }
            (_, actual) => Err(AccessErrorKind::invalid_type(TypeKind::Struct, actual)),
        }
    }
}

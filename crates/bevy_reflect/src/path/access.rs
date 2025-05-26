//! Representation for individual element accesses within a path.

use alloc::borrow::Cow;
use core::fmt;

use super::error::AccessErrorKind;
use crate::{AccessError, PartialReflect, ReflectKind, ReflectMut, ReflectRef, VariantType};

type InnerResult<'a, T> = Result<T, AccessErrorKind<'a>>;

/// A singular element access within a path.
/// Multiple accesses can be combined into a [`ParsedPath`](super::ParsedPath).
///
/// Can be applied to a [`dyn Reflect`](crate::Reflect) to get a reference to the targeted element.
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
    /// the field's [`Cow<str>`] will be converted to its owned
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
        base: &'r dyn PartialReflect,
        offset: Option<usize>,
    ) -> Result<&'r dyn PartialReflect, AccessError<'a>> {
        self.element_inner(base)
            .and_then(|opt| opt.ok_or(AccessErrorKind::MissingField(base.reflect_kind())))
            .map_err(|err| err.with_access(self.clone(), offset))
    }

    #[inline]
    fn try_parse_field_as_index(
        base: impl Fn() -> ReflectKind,
        field: Cow<'a, str>,
    ) -> Result<usize, AccessErrorKind<'a>> {
        field
            .parse()
            .map_err(|_| AccessErrorKind::UnsupportedAccess {
                base: base(),
                access: Access::Field(field),
            })
    }

    fn element_inner<'r>(
        &self,
        base: &'r dyn PartialReflect,
    ) -> InnerResult<'a, Option<&'r dyn PartialReflect>> {
        use ReflectRef::*;

        let invalid_variant =
            |expected, actual| AccessErrorKind::IncompatibleEnumVariantTypes { expected, actual };

        match (base.reflect_ref(), self) {
            // Struct
            (Struct(struct_ref), Access::Field(field)) => Ok(struct_ref.field(field.as_ref())),
            (Struct(struct_ref), &Access::FieldIndex(index)) => Ok(struct_ref.field_at(index)),
            // Tuple Struct
            (TupleStruct(struct_ref), Access::Field(field)) => Ok(struct_ref.field(
                Self::try_parse_field_as_index(|| struct_ref.reflect_kind(), field.clone())?,
            )),
            (
                TupleStruct(struct_ref),
                &Access::FieldIndex(index) | &Access::TupleIndex(index) | &Access::ListIndex(index),
            ) => Ok(struct_ref.field(index)),
            // Tuple
            (Tuple(tuple_ref), Access::Field(field)) => Ok(tuple_ref.field(
                Self::try_parse_field_as_index(|| tuple_ref.reflect_kind(), field.clone())?,
            )),
            (
                Tuple(tuple_ref),
                &Access::FieldIndex(index) | &Access::TupleIndex(index) | &Access::ListIndex(index),
            ) => Ok(tuple_ref.field(index)),
            // List
            (List(list_ref), Access::Field(field)) => Ok(list_ref.get(
                Self::try_parse_field_as_index(|| list_ref.reflect_kind(), field.clone())?,
            )),
            (
                List(list_ref),
                &Access::FieldIndex(index) | &Access::TupleIndex(index) | &Access::ListIndex(index),
            ) => Ok(list_ref.get(index)),
            // Array
            (Array(array_ref), Access::Field(field)) => Ok(array_ref.get(
                Self::try_parse_field_as_index(|| array_ref.reflect_kind(), field.clone())?,
            )),
            (
                Array(array_ref),
                &Access::FieldIndex(index) | &Access::TupleIndex(index) | &Access::ListIndex(index),
            ) => Ok(array_ref.get(index)),
            // Map
            (Map(map_ref), Access::Field(field)) => Ok(map_ref.get(&field.clone().into_owned())),
            (
                Map(map_ref),
                &Access::FieldIndex(index) | &Access::TupleIndex(index) | &Access::ListIndex(index),
            ) => Ok(map_ref.get(&index)),
            // Set
            (Set(set_ref), Access::Field(field)) => Ok(set_ref.get(&field.clone().into_owned())),
            (
                Set(set_ref),
                &Access::FieldIndex(index) | &Access::TupleIndex(index) | &Access::ListIndex(index),
            ) => Ok(set_ref.get(&index)),
            // Enum
            (Enum(enum_ref), Access::Field(field)) => match enum_ref.variant_type() {
                VariantType::Struct => Ok(enum_ref.field(field.as_ref())),
                actual => Err(invalid_variant(VariantType::Struct, actual)),
            },
            (Enum(enum_ref), &Access::FieldIndex(index)) => match enum_ref.variant_type() {
                VariantType::Struct => Ok(enum_ref.field_at(index)),
                actual => Err(invalid_variant(VariantType::Struct, actual)),
            },
            (Enum(enum_ref), &Access::TupleIndex(index) | &Access::ListIndex(index)) => {
                match enum_ref.variant_type() {
                    VariantType::Tuple => Ok(enum_ref.field_at(index)),
                    actual => Err(invalid_variant(VariantType::Tuple, actual)),
                }
            }
            (other, access) => Err(AccessErrorKind::UnsupportedAccess {
                base: other.kind(),
                access: access.clone().into_owned(),
            }),
        }
    }

    pub(super) fn element_mut<'r>(
        &self,
        base: &'r mut dyn PartialReflect,
        offset: Option<usize>,
    ) -> Result<&'r mut dyn PartialReflect, AccessError<'a>> {
        let kind = base.reflect_kind();

        self.element_inner_mut(base)
            .and_then(|maybe| maybe.ok_or(AccessErrorKind::MissingField(kind)))
            .map_err(move |err| err.with_access(self.clone(), offset))
    }

    fn element_inner_mut<'r>(
        &self,
        base: &'r mut dyn PartialReflect,
    ) -> InnerResult<'a, Option<&'r mut dyn PartialReflect>> {
        use ReflectMut::*;

        let invalid_variant =
            |expected, actual| AccessErrorKind::IncompatibleEnumVariantTypes { expected, actual };

        match (base.reflect_mut(), self) {
            // Struct
            (Struct(struct_ref), Access::Field(field)) => Ok(struct_ref.field_mut(field.as_ref())),
            (
                Struct(struct_ref),
                &Access::FieldIndex(index) | &Access::TupleIndex(index) | &Access::ListIndex(index),
            ) => Ok(struct_ref.field_at_mut(index)),
            // Tuple Struct
            (TupleStruct(struct_ref), Access::Field(field)) => Ok(struct_ref.field_mut(
                Self::try_parse_field_as_index(|| struct_ref.reflect_kind(), field.clone())?,
            )),
            (
                TupleStruct(struct_ref),
                &Access::FieldIndex(index) | &Access::TupleIndex(index) | &Access::ListIndex(index),
            ) => Ok(struct_ref.field_mut(index)),
            // Tuple
            (Tuple(tuple_ref), Access::Field(field)) => Ok(tuple_ref.field_mut(
                Self::try_parse_field_as_index(|| tuple_ref.reflect_kind(), field.clone())?,
            )),
            (
                Tuple(tuple_ref),
                &Access::FieldIndex(index) | &Access::TupleIndex(index) | &Access::ListIndex(index),
            ) => Ok(tuple_ref.field_mut(index)),
            // List
            (List(list_ref), Access::Field(field)) => Ok(list_ref.get_mut(
                Self::try_parse_field_as_index(|| list_ref.reflect_kind(), field.clone())?,
            )),
            (
                List(list_ref),
                &Access::FieldIndex(index) | &Access::TupleIndex(index) | &Access::ListIndex(index),
            ) => Ok(list_ref.get_mut(index)),
            // Array
            (Array(array_ref), Access::Field(field)) => Ok(array_ref.get_mut(
                Self::try_parse_field_as_index(|| array_ref.reflect_kind(), field.clone())?,
            )),
            (
                Array(array_ref),
                &Access::FieldIndex(index) | &Access::TupleIndex(index) | &Access::ListIndex(index),
            ) => Ok(array_ref.get_mut(index)),
            // Map
            (Map(map_ref), Access::Field(field)) => {
                Ok(map_ref.get_mut(&field.clone().into_owned()))
            }
            (
                Map(map_ref),
                &Access::FieldIndex(index) | &Access::TupleIndex(index) | &Access::ListIndex(index),
            ) => Ok(map_ref.get_mut(&index)),
            // Set - no get_mut
            // (Set(set_ref), Access::Field(field)) => Ok(set_ref.get(&field.clone().into_owned())),
            // (
            //     Set(set_ref),
            //     &Access::FieldIndex(index) | &Access::TupleIndex(index) | &Access::ListIndex(index),
            // ) => Ok(set_ref.get(&index)),
            // Enum
            // Enum
            (Enum(enum_ref), Access::Field(field)) => match enum_ref.variant_type() {
                VariantType::Struct => Ok(enum_ref.field_mut(field.as_ref())),
                actual => Err(invalid_variant(VariantType::Struct, actual)),
            },
            (Enum(enum_ref), &Access::FieldIndex(index)) => match enum_ref.variant_type() {
                VariantType::Struct => Ok(enum_ref.field_at_mut(index)),
                actual => Err(invalid_variant(VariantType::Struct, actual)),
            },
            (Enum(enum_ref), &Access::TupleIndex(index) | &Access::ListIndex(index)) => {
                match enum_ref.variant_type() {
                    VariantType::Tuple => Ok(enum_ref.field_at_mut(index)),
                    actual => Err(invalid_variant(VariantType::Tuple, actual)),
                }
            }
            (other, access) => Err(AccessErrorKind::UnsupportedAccess {
                base: other.kind(),
                access: access.clone().into_owned(),
            }),
        }
    }

    /// Returns a reference to this [`Access`]'s inner value as a [`&dyn Display`](fmt::Display).
    pub fn display_value(&self) -> &dyn fmt::Display {
        match self {
            Self::Field(value) => value,
            Self::FieldIndex(value) | Self::TupleIndex(value) | Self::ListIndex(value) => value,
        }
    }

    pub(super) fn kind(&self) -> &'static str {
        match self {
            Self::Field(_) => "field",
            Self::FieldIndex(_) => "field index",
            Self::TupleIndex(_) | Self::ListIndex(_) => "index",
        }
    }
}

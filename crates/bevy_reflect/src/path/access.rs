//! Representation for individual element accesses within a path.

use alloc::borrow::Cow;
use core::fmt::{self};

use super::error::AccessErrorKind;
use crate::{enums::VariantType, AccessError, PartialReflect, ReflectKind, ReflectMut, ReflectRef};

type InnerResult<T> = Result<T, AccessErrorKind>;

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
    /// Variant index-based access for an enum.
    Variant(VariantAccess<'a>),
}
/// Field Access for Enum variants.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VariantAccess<'a> {
    /// Access for a unit enum variant - ex: `Option::None`
    Unit(usize),
    /// Access for a struct enum variant, keyed on field name.
    Field(usize, Cow<'a, str>),
    /// Access for a struct enum variant, keyed on field index.
    FieldIndex(usize, usize),
    /// Access for a Tuple enum variant, keyed on tuple index.
    TupleIndex(usize, usize),
}

impl fmt::Display for Access<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Access::Field(field) => write!(f, ".{field}"),
            Access::FieldIndex(index) => write!(f, "#{index}"),
            Access::TupleIndex(index) => write!(f, ".{index}"),
            Access::ListIndex(index) => write!(f, "[{index}]"),
            Access::Variant(index) => write!(f, "{}", index),
        }
    }
}
impl fmt::Display for VariantAccess<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VariantAccess::Unit(vidx) => write!(f, "{{{vidx}}}"),
            VariantAccess::Field(vidx, field) => write!(f, "{{{vidx}.{field}}}"),
            VariantAccess::FieldIndex(vidx, index) => write!(f, "{{{vidx}#{index}}}"),
            VariantAccess::TupleIndex(vidx, index) => write!(f, "{{{vidx}.{index}}}"),
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
        use VariantAccess::*;
        match self {
            Self::Field(value) => Access::Field(Cow::Owned(value.into_owned())),
            Self::FieldIndex(value) => Access::FieldIndex(value),
            Self::TupleIndex(value) => Access::TupleIndex(value),
            Self::ListIndex(value) => Access::ListIndex(value),
            Self::Variant(Unit(v)) => Access::Variant(Unit(v)),
            Self::Variant(Field(v_index, field)) => {
                Access::Variant(Field(v_index, Cow::Owned(field.into_owned())))
            }
            Self::Variant(FieldIndex(v_index, index)) => {
                Access::Variant(FieldIndex(v_index, index))
            }
            Self::Variant(TupleIndex(v_index, index)) => {
                Access::Variant(TupleIndex(v_index, index))
            }
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

    fn element_inner<'r>(
        &self,
        base: &'r dyn PartialReflect,
    ) -> InnerResult<Option<&'r dyn PartialReflect>> {
        use ReflectRef::*;
        use VariantAccess::*;

        let invalid_variant =
            |expected, actual| AccessErrorKind::IncompatibleEnumVariantTypes { expected, actual };

        match (self, base.reflect_ref()) {
            (Self::Field(field), Struct(struct_ref)) => Ok(struct_ref.field(field.as_ref())),
            (&Self::FieldIndex(index), Struct(struct_ref)) => Ok(struct_ref.field_at(index)),
            (Self::Field(_) | Self::FieldIndex(_), actual) => {
                Err(AccessErrorKind::IncompatibleTypes {
                    expected: ReflectKind::Struct,
                    actual: actual.into(),
                })
            }

            (&Self::TupleIndex(index), TupleStruct(tuple)) => Ok(tuple.field(index)),
            (&Self::TupleIndex(index), Tuple(tuple)) => Ok(tuple.field(index)),

            (Self::TupleIndex(_), actual) => Err(AccessErrorKind::IncompatibleTypes {
                expected: ReflectKind::Tuple,
                actual: actual.into(),
            }),

            (&Self::ListIndex(index), List(list)) => Ok(list.get(index)),
            (&Self::ListIndex(index), Array(list)) => Ok(list.get(index)),
            (Self::ListIndex(_), actual) => Err(AccessErrorKind::IncompatibleTypes {
                expected: ReflectKind::List,
                actual: actual.into(),
            }),
            (Self::Variant(Field(index, field)), Enum(enum_ref)) => {
                if enum_ref.variant_index() != *index {
                    Err(AccessErrorKind::IncorrectEnumVariantIndex {
                        expected: *index,
                        actual: enum_ref.variant_index(),
                    })
                } else {
                    match enum_ref.variant_type() {
                        VariantType::Struct => Ok(enum_ref.field(field.as_ref())),
                        actual => Err(invalid_variant(VariantType::Struct, actual)),
                    }
                }
            }
            (&Self::Variant(FieldIndex(v_index, index)), Enum(enum_ref)) => {
                if enum_ref.variant_index() != v_index {
                    Err(AccessErrorKind::IncorrectEnumVariantIndex {
                        expected: v_index,
                        actual: enum_ref.variant_index(),
                    })
                } else {
                    match enum_ref.variant_type() {
                        VariantType::Struct => Ok(enum_ref.field_at(index)),
                        actual => Err(invalid_variant(VariantType::Struct, actual)),
                    }
                }
            }
            (&Self::Variant(TupleIndex(v_index, index)), Enum(enum_ref)) => {
                if enum_ref.variant_index() != v_index {
                    Err(AccessErrorKind::IncorrectEnumVariantIndex {
                        expected: v_index,
                        actual: enum_ref.variant_index(),
                    })
                } else {
                    match enum_ref.variant_type() {
                        VariantType::Tuple => Ok(enum_ref.field_at(index)),
                        actual => Err(invalid_variant(VariantType::Tuple, actual)),
                    }
                }
            }
            (&Self::Variant(Unit(index)), Enum(enum_ref)) => {
                if enum_ref.variant_index() == index {
                    Ok(Some(enum_ref.as_partial_reflect()))
                } else {
                    Err(AccessErrorKind::IncorrectEnumVariantIndex {
                        expected: index,
                        actual: enum_ref.variant_index(),
                    })
                }
            }
            (&Self::Variant(_), actual) => Err(AccessErrorKind::IncompatibleTypes {
                expected: ReflectKind::Enum,
                actual: actual.into(),
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
            .map_err(|err| err.with_access(self.clone(), offset))
    }

    fn element_inner_mut<'r>(
        &self,
        base: &'r mut dyn PartialReflect,
    ) -> InnerResult<Option<&'r mut dyn PartialReflect>> {
        use ReflectMut::*;
        use VariantAccess::*;

        let invalid_variant =
            |expected, actual| AccessErrorKind::IncompatibleEnumVariantTypes { expected, actual };

        match (self, base.reflect_mut()) {
            (Self::Field(field), Struct(struct_mut)) => Ok(struct_mut.field_mut(field.as_ref())),
            (&Self::FieldIndex(index), Struct(struct_mut)) => Ok(struct_mut.field_at_mut(index)),
            (Self::Field(_) | Self::FieldIndex(_), actual) => {
                Err(AccessErrorKind::IncompatibleTypes {
                    expected: ReflectKind::Struct,
                    actual: actual.into(),
                })
            }

            (&Self::TupleIndex(index), TupleStruct(tuple)) => Ok(tuple.field_mut(index)),
            (&Self::TupleIndex(index), Tuple(tuple)) => Ok(tuple.field_mut(index)),
            (Self::TupleIndex(_), actual) => Err(AccessErrorKind::IncompatibleTypes {
                expected: ReflectKind::Tuple,
                actual: actual.into(),
            }),

            (&Self::ListIndex(index), List(list)) => Ok(list.get_mut(index)),
            (&Self::ListIndex(index), Array(list)) => Ok(list.get_mut(index)),
            (Self::ListIndex(_), actual) => Err(AccessErrorKind::IncompatibleTypes {
                expected: ReflectKind::List,
                actual: actual.into(),
            }),
            (Self::Variant(Field(index, field)), Enum(enum_ref)) => {
                if enum_ref.variant_index() != *index {
                    Err(AccessErrorKind::IncorrectEnumVariantIndex {
                        expected: *index,
                        actual: enum_ref.variant_index(),
                    })
                } else {
                    match enum_ref.variant_type() {
                        VariantType::Struct => Ok(enum_ref.field_mut(field.as_ref())),
                        actual => Err(invalid_variant(VariantType::Struct, actual)),
                    }
                }
            }
            (&Self::Variant(FieldIndex(v_index, index)), Enum(enum_ref)) => {
                if enum_ref.variant_index() != v_index {
                    Err(AccessErrorKind::IncorrectEnumVariantIndex {
                        expected: v_index,
                        actual: enum_ref.variant_index(),
                    })
                } else {
                    match enum_ref.variant_type() {
                        VariantType::Struct => Ok(enum_ref.field_at_mut(index)),
                        actual => Err(invalid_variant(VariantType::Struct, actual)),
                    }
                }
            }
            (&Self::Variant(TupleIndex(v_index, index)), Enum(enum_ref)) => {
                if enum_ref.variant_index() != v_index {
                    Err(AccessErrorKind::IncorrectEnumVariantIndex {
                        expected: v_index,
                        actual: enum_ref.variant_index(),
                    })
                } else {
                    match enum_ref.variant_type() {
                        VariantType::Tuple => Ok(enum_ref.field_at_mut(index)),
                        actual => Err(invalid_variant(VariantType::Tuple, actual)),
                    }
                }
            }
            (&Self::Variant(Unit(index)), Enum(enum_ref)) => {
                if enum_ref.variant_index() == index {
                    Ok(Some(enum_ref.as_partial_reflect_mut()))
                } else {
                    Err(AccessErrorKind::IncorrectEnumVariantIndex {
                        expected: index,
                        actual: enum_ref.variant_index(),
                    })
                }
            }
            (&Self::Variant(_), actual) => Err(AccessErrorKind::IncompatibleTypes {
                expected: ReflectKind::Enum,
                actual: actual.into(),
            }),
        }
    }

    /// Returns a reference to this [`Access`]'s inner value as a [`&dyn Display`](fmt::Display).
    pub fn display_value(&self) -> &dyn fmt::Display {
        match self {
            Self::Field(value) => value,
            Self::FieldIndex(value) | Self::TupleIndex(value) | Self::ListIndex(value) => value,
            Self::Variant(value) => value,
        }
    }

    pub(super) fn kind(&self) -> &'static str {
        use VariantAccess::*;
        match self {
            Self::Field(_) => "field",
            Self::FieldIndex(_) => "field index",
            Self::TupleIndex(_) | Self::ListIndex(_) => "index",
            Self::Variant(Unit(_)) => "unit variant index",
            Self::Variant(FieldIndex(_, _)) => "variant index with field index",
            Self::Variant(TupleIndex(_, _)) => "variant index with tuple index",
            Self::Variant(Field(_, _)) => "variant index with field",
        }
    }
}

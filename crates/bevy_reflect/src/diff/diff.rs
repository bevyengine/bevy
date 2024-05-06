use crate::diff::{
    ArrayDiff, DiffApplyError, DiffError, EnumDiff, ListDiff, MapDiff, StructDiff, TupleDiff,
    TupleStructDiff, ValueDiff,
};
use crate::{Reflect, ReflectKind, ReflectMut, TypeInfo};
use std::ops::Deref;

/// Indicates the difference between two [`Reflect`] objects.
///
/// [`Reflect`]: crate::Reflect
#[derive(Debug)]
pub enum Diff<'old, 'new> {
    /// Indicates no change.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::{Reflect, diff::Diff};
    /// let old = 123;
    /// let new = 123;
    ///
    /// let diff = old.diff(&new).unwrap();
    /// assert!(matches!(diff, Diff::NoChange));
    /// ```
    ///
    NoChange,
    /// Indicates that the type has been changed.
    ///
    /// Contains the "new" value.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::{Reflect, diff::Diff};
    /// let old: bool = true;
    /// let new: i32 = 123;
    ///
    /// let diff = old.diff(&new).unwrap();
    /// assert!(matches!(diff, Diff::Replaced(..)));
    /// ```
    ///
    Replaced(ValueDiff<'new>),
    /// Indicates that the value has been modified.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::{Reflect, diff::Diff};
    /// let old: i32 = 123;
    /// let new: i32 = 456;
    ///
    /// let diff = old.diff(&new).unwrap();
    /// assert!(matches!(diff, Diff::Modified(..)));
    /// ```
    ///
    Modified(DiffType<'old, 'new>),
}

impl<'old, 'new> Diff<'old, 'new> {
    /// Apply this `Diff` to the given [`Reflect`] object.
    ///
    /// # Errors
    ///
    /// Returns an error if the diff cannot be applied to the given object.
    ///
    /// Note that this may leave the object in an invalid state.
    /// If a possible error is expected, it is recommended to keep a copy of the original object,
    /// such that it can be restored if necessary.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::Reflect;
    /// let old = vec![1, 2, 3];
    /// let new = vec![0, 2, 4];
    ///
    /// let diff = old.diff(&new).unwrap();
    ///
    /// let mut value = vec![1, 2, 3];
    /// diff.apply(&mut value).unwrap();
    /// ```
    pub fn apply(self, onto: &mut dyn Reflect) -> DiffApplyResult {
        let diff = match self {
            Self::NoChange => return Ok(()),
            Self::Replaced(_) => return Err(DiffApplyError::TypeMismatch),
            Self::Modified(diff_type) => diff_type,
        };

        let onto = onto.reflect_mut();

        return match (onto, diff) {
            (ReflectMut::Value(value), DiffType::Value(diff)) => {
                value.try_apply(diff.deref()).map_err(Into::into)
            }
            (ReflectMut::Tuple(value), DiffType::Tuple(diff)) => value.apply_tuple_diff(diff),
            (ReflectMut::Array(value), DiffType::Array(diff)) => value.apply_array_diff(diff),
            (ReflectMut::List(value), DiffType::List(diff)) => value.apply_list_diff(diff),
            (ReflectMut::Map(value), DiffType::Map(diff)) => value.apply_map_diff(diff),
            (ReflectMut::TupleStruct(value), DiffType::TupleStruct(diff)) => {
                value.apply_tuple_struct_diff(diff)
            }
            (ReflectMut::Struct(value), DiffType::Struct(diff)) => value.apply_struct_diff(diff),
            (ReflectMut::Enum(value), DiffType::Enum(diff)) => value.apply_enum_diff(diff),
            (onto, diff) => Err(DiffApplyError::KindMismatch {
                expected: diff.kind(),
                received: onto.kind(),
            }),
        };
    }

    pub fn clone_diff(&self) -> Diff<'static, 'static> {
        match self {
            Self::NoChange => Diff::NoChange,
            Self::Replaced(value_diff) => Diff::Replaced(value_diff.clone_diff()),
            Self::Modified(diff_type) => Diff::Modified(diff_type.clone_diff()),
        }
    }
}

/// Contains diffing details for each [reflection type].
///
/// [reflection type]: crate::ReflectRef
#[derive(Debug)]
pub enum DiffType<'old, 'new> {
    Value(ValueDiff<'new>),
    Tuple(TupleDiff<'old, 'new>),
    Array(ArrayDiff<'old, 'new>),
    List(ListDiff<'new>),
    Map(MapDiff<'old, 'new>),
    TupleStruct(TupleStructDiff<'old, 'new>),
    Struct(StructDiff<'old, 'new>),
    Enum(EnumDiff<'old, 'new>),
}

impl<'old, 'new> DiffType<'old, 'new> {
    /// Returns the [`TypeInfo`] of the reflected value currently being diffed.
    pub fn type_info(&self) -> &TypeInfo {
        match self {
            DiffType::Value(value_diff) => value_diff.type_info(),
            DiffType::Tuple(tuple_diff) => tuple_diff.type_info(),
            DiffType::Array(array_diff) => array_diff.type_info(),
            DiffType::List(list_diff) => list_diff.type_info(),
            DiffType::Map(map_diff) => map_diff.type_info(),
            DiffType::TupleStruct(tuple_struct_diff) => tuple_struct_diff.type_info(),
            DiffType::Struct(struct_diff) => struct_diff.type_info(),
            DiffType::Enum(enum_diff) => enum_diff.type_info(),
        }
    }

    pub fn clone_diff(&self) -> DiffType<'static, 'static> {
        match self {
            DiffType::Value(value_diff) => DiffType::Value(value_diff.clone_diff()),
            DiffType::Tuple(tuple_diff) => DiffType::Tuple(tuple_diff.clone_diff()),
            DiffType::Array(array_diff) => DiffType::Array(array_diff.clone_diff()),
            DiffType::List(list_diff) => DiffType::List(list_diff.clone_diff()),
            DiffType::Map(map_diff) => DiffType::Map(map_diff.clone_diff()),
            DiffType::TupleStruct(tuple_struct_diff) => {
                DiffType::TupleStruct(tuple_struct_diff.clone_diff())
            }
            DiffType::Struct(struct_diff) => DiffType::Struct(struct_diff.clone_diff()),
            DiffType::Enum(enum_diff) => DiffType::Enum(enum_diff.clone_diff()),
        }
    }

    /// Returns the [kind] of this diff.
    ///
    /// [kind]: ReflectKind
    pub fn kind(&self) -> ReflectKind {
        match self {
            DiffType::Value(_) => ReflectKind::Value,
            DiffType::Tuple(_) => ReflectKind::Tuple,
            DiffType::Array(_) => ReflectKind::Array,
            DiffType::List(_) => ReflectKind::List,
            DiffType::Map(_) => ReflectKind::Map,
            DiffType::TupleStruct(_) => ReflectKind::TupleStruct,
            DiffType::Struct(_) => ReflectKind::Struct,
            DiffType::Enum(_) => ReflectKind::Enum,
        }
    }
}

/// Alias for a `Result` that returns either [`Ok(Diff)`](Diff) or [`Err(DiffError)`](DiffError).
///
/// This is most commonly used by the [`Reflect::diff`] method as well as the utility functions
/// provided in this module.
pub type DiffResult<'old, 'new> = Result<Diff<'old, 'new>, DiffError>;

/// Alias for a `Result` that returns either `Ok(())` or [`Err(DiffApplyError)`](DiffApplyError).
///
/// This is most commonly used by the [`Reflect::apply_diff`] method as well as the utility functions
/// provided in this module.
pub type DiffApplyResult = Result<(), DiffApplyError>;

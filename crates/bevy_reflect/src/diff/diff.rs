use crate::diff::{
    ArrayDiff, DiffError, EnumDiff, ListDiff, MapDiff, StructDiff, TupleDiff, TupleStructDiff,
    ValueDiff,
};
use crate::{Reflect, TypeInfo};

/// Indicates the difference between two [`Reflect`] objects.
///
/// [`Reflect`]: crate::Reflect
#[derive(Debug)]
pub enum Diff<'old, 'new> {
    /// Indicates no change.
    ///
    /// Contains the "old" value.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::{Reflect, diff::Diff};
    /// let old = 123;
    /// let new = 123;
    ///
    /// let diff = old.diff(&new).unwrap();
    /// assert!(matches!(diff, Diff::NoChange(_)));
    /// ```
    ///
    NoChange(&'old dyn Reflect),
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
}

/// Alias for a `Result` that returns either [`Ok(Diff)`](Diff) or [`Err(DiffError)`](DiffError).
///
/// This is most commonly used by the [`Reflect::diff`] method as well as the utility functions
/// provided in this module.
///
/// [`Reflect::diff`]: crate::Reflect::diff
pub type DiffResult<'old, 'new> = Result<Diff<'old, 'new>, DiffError>;

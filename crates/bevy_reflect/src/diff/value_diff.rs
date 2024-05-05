use crate::diff::{Diff, DiffError, DiffResult, DiffType};
use crate::{Reflect, ReflectKind, TypeInfo};

use std::ops::Deref;

/// Represents a plain value in a [`Diff`](crate::diff::Diff).
///
/// This can contain either an owned [`Reflect`] object or an immutable/mutable reference to one.
#[derive(Debug)]
pub enum ValueDiff<'a> {
    Borrowed(&'a dyn Reflect),
    Owned(Box<dyn Reflect>),
}

impl<'a> ValueDiff<'a> {
    /// Returns the [`TypeInfo`] of the reflected value currently being diffed.
    pub fn type_info(&self) -> &TypeInfo {
        self.get_represented_type_info()
            .expect("reflected value type should have TypeInfo")
    }
}

impl<'a> Deref for ValueDiff<'a> {
    type Target = dyn Reflect;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Borrowed(value) => *value,
            Self::Owned(value) => value.as_ref(),
        }
    }
}

impl<'a> From<&'a dyn Reflect> for ValueDiff<'a> {
    fn from(value: &'a dyn Reflect) -> Self {
        Self::Borrowed(value)
    }
}

impl<'a> From<Box<dyn Reflect>> for ValueDiff<'a> {
    fn from(value: Box<dyn Reflect>) -> Self {
        Self::Owned(value)
    }
}

/// Utility function for diffing two [`Reflect`] objects.
///
/// This should be used for [value] types such as primitives.
/// For structs, enums, and other data structures, see the similar methods in the [diff] module.
///
/// [value]: crate::ReflectRef::Value
/// [diff]: crate::diff
pub fn diff_value<'old, 'new>(
    old: &'old dyn Reflect,
    new: &'new dyn Reflect,
) -> DiffResult<'old, 'new> {
    match (old.reflect_kind(), new.reflect_kind()) {
        (ReflectKind::Value, ReflectKind::Value) => {
            if old.type_id() != new.type_id() {
                return Ok(Diff::Replaced(ValueDiff::Borrowed(new)));
            }

            match old.reflect_partial_eq(new) {
                Some(true) => Ok(Diff::NoChange(old)),
                Some(false) => Ok(Diff::Modified(DiffType::Value(ValueDiff::Borrowed(new)))),
                None => Err(DiffError::Incomparable),
            }
        }
        (expected, received) => Err(DiffError::KindMismatch { expected, received }),
    }
}

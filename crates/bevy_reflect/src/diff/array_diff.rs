use crate::diff::{Diff, DiffError, DiffResult, DiffType, ValueDiff};
use crate::{Array, Reflect, ReflectKind, ReflectRef, TypeInfo};
use std::fmt::{Debug, Formatter};
use std::slice::Iter;

/// Diff object for [arrays](Array).
pub struct ArrayDiff<'old, 'new> {
    type_info: &'static TypeInfo,
    elements: Vec<Diff<'old, 'new>>,
}

impl<'old, 'new> ArrayDiff<'old, 'new> {
    /// Returns the [`TypeInfo`] of the reflected value currently being diffed.
    pub fn type_info(&self) -> &TypeInfo {
        self.type_info
    }

    /// Returns the [`Diff`] for the field at the given index.
    pub fn get(&self, index: usize) -> Option<&Diff<'old, 'new>> {
        self.elements.get(index)
    }

    /// Returns the number of elements in the array.
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Returns true if the array contains no elements.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Returns an iterator over the [`Diff`] for _every_ element.
    pub fn iter(&self) -> Iter<'_, Diff<'old, 'new>> {
        self.elements.iter()
    }

    /// Take the changes contained in this diff.
    pub fn take_changes(self) -> Vec<Diff<'old, 'new>> {
        self.elements
    }

    pub fn clone_diff(&self) -> ArrayDiff<'static, 'static> {
        ArrayDiff {
            type_info: self.type_info,
            elements: self.elements.iter().map(Diff::clone_diff).collect(),
        }
    }
}

impl<'old, 'new> Debug for ArrayDiff<'old, 'new> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArrayDiff")
            .field("elements", &self.elements)
            .finish()
    }
}

/// Utility function for diffing two [`Array`] objects.
pub fn diff_array<'old, 'new, T: Array>(
    old: &'old T,
    new: &'new dyn Reflect,
) -> DiffResult<'old, 'new> {
    let new = match new.reflect_ref() {
        ReflectRef::Array(new) => new,
        new => {
            return Err(DiffError::KindMismatch {
                expected: ReflectKind::Array,
                received: new.kind(),
            })
        }
    };

    let (old_info, new_info) = old
        .get_represented_type_info()
        .zip(new.get_represented_type_info())
        .ok_or(DiffError::MissingInfo)?;

    if old.len() != new.len() || old_info.type_id() != new_info.type_id() {
        return Ok(Diff::Replaced(ValueDiff::Borrowed(new.as_reflect())));
    }

    let mut diff = ArrayDiff {
        type_info: old_info,
        elements: Vec::with_capacity(old.len()),
    };

    let mut was_modified = false;
    for (old_field, new_field) in old.iter().zip(new.iter()) {
        let field_diff = old_field.diff(new_field)?;
        was_modified |= !matches!(field_diff, Diff::NoChange);
        diff.elements.push(field_diff);
    }

    if was_modified {
        Ok(Diff::Modified(DiffType::Array(diff)))
    } else {
        Ok(Diff::NoChange)
    }
}

use crate::diff::{Diff, DiffError, DiffResult, DiffType, ValueDiff};
use crate::{Reflect, ReflectKind, ReflectRef, TupleStruct, TypeInfo};
use std::fmt::{Debug, Formatter};
use std::slice::Iter;

/// Diff object for [tuple structs](TupleStruct).
pub struct TupleStructDiff<'old, 'new> {
    type_info: &'static TypeInfo,
    fields: Vec<Diff<'old, 'new>>,
}

impl<'old, 'new> TupleStructDiff<'old, 'new> {
    /// Returns the [`TypeInfo`] of the reflected value currently being diffed.
    pub fn type_info(&self) -> &TypeInfo {
        self.type_info
    }

    /// Returns the [`Diff`] for the field at the given index.
    pub fn field(&self, index: usize) -> Option<&Diff<'old, 'new>> {
        self.fields.get(index)
    }

    /// Returns the number of fields in the tuple struct.
    pub fn field_len(&self) -> usize {
        self.fields.len()
    }

    /// Returns an iterator over the [`Diff`] for _every_ field.
    pub fn field_iter(&self) -> Iter<'_, Diff<'old, 'new>> {
        self.fields.iter()
    }

    /// Take the changes contained in this diff.
    pub fn take_changes(self) -> Vec<Diff<'old, 'new>> {
        self.fields
    }
}

impl<'old, 'new> Debug for TupleStructDiff<'old, 'new> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TupleStructDiff")
            .field("fields", &self.fields)
            .finish()
    }
}

/// Utility function for diffing two [`TupleStruct`] objects.
pub fn diff_tuple_struct<'old, 'new, T: TupleStruct>(
    old: &'old T,
    new: &'new dyn Reflect,
) -> DiffResult<'old, 'new> {
    let new = match new.reflect_ref() {
        ReflectRef::TupleStruct(new) => new,
        new => {
            return Err(DiffError::KindMismatch {
                expected: ReflectKind::TupleStruct,
                received: new.kind(),
            })
        }
    };

    let (old_info, new_info) = old
        .get_represented_type_info()
        .zip(new.get_represented_type_info())
        .ok_or(DiffError::MissingInfo)?;

    if old.field_len() != new.field_len() || old_info.type_id() != new_info.type_id() {
        return Ok(Diff::Replaced(ValueDiff::Borrowed(new.as_reflect())));
    }

    let mut diff = TupleStructDiff {
        type_info: old_info,
        fields: Vec::with_capacity(old.field_len()),
    };

    let mut was_modified = false;
    for (old_field, new_field) in old.iter_fields().zip(new.iter_fields()) {
        let field_diff = old_field.diff(new_field)?;
        was_modified |= !matches!(field_diff, Diff::NoChange(_));
        diff.fields.push(field_diff);
    }

    if was_modified {
        Ok(Diff::Modified(DiffType::TupleStruct(diff)))
    } else {
        Ok(Diff::NoChange(old))
    }
}

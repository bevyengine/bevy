use crate::diff::{Diff, DiffError, DiffResult, DiffType};
use crate::{Reflect, ReflectKind, ReflectRef, Tuple};
use std::fmt::{Debug, Formatter};
use std::slice::Iter;

/// Diff object for (tuples)[Tuple].
#[derive(Clone)]
pub struct DiffedTuple<'old, 'new> {
    new_value: &'new dyn Tuple,
    fields: Vec<Diff<'old, 'new>>,
}

impl<'old, 'new> DiffedTuple<'old, 'new> {
    /// Returns the "new" tuple value.
    pub fn new_value(&self) -> &'new dyn Tuple {
        self.new_value
    }

    /// Returns the [`Diff`] for the field at the given index.
    pub fn field(&self, index: usize) -> Option<&Diff<'old, 'new>> {
        self.fields.get(index)
    }

    /// Returns the number of fields in the tuple.
    pub fn field_len(&self) -> usize {
        self.fields.len()
    }

    /// Returns an iterator over the [`Diff`] for _every_ field.
    pub fn field_iter(&self) -> Iter<'_, Diff<'old, 'new>> {
        self.fields.iter()
    }
}

impl<'old, 'new> Debug for DiffedTuple<'old, 'new> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiffedTuple")
            .field("fields", &self.fields)
            .finish()
    }
}

/// Utility function for diffing two [`Tuple`] objects.
pub fn diff_tuple<'old, 'new, T: Tuple>(
    old: &'old T,
    new: &'new dyn Reflect,
) -> DiffResult<'old, 'new> {
    let new = match new.reflect_ref() {
        ReflectRef::Tuple(new) => new,
        new => {
            return Err(DiffError::KindMismatch {
                expected: ReflectKind::Tuple,
                received: new.kind(),
            })
        }
    };

    let (old_info, new_info) = old
        .get_represented_type_info()
        .zip(new.get_represented_type_info())
        .ok_or(DiffError::MissingInfo)?;

    if old.field_len() != new.field_len() || old_info.type_id() != new_info.type_id() {
        return Ok(Diff::Replaced(new.as_reflect()));
    }

    let mut diff = DiffedTuple {
        new_value: new,
        fields: Vec::with_capacity(old.field_len()),
    };

    let mut was_modified = false;
    for (old_field, new_field) in old.iter_fields().zip(new.iter_fields()) {
        let field_diff = old_field.diff(new_field)?;
        was_modified |= !matches!(field_diff, Diff::NoChange);
        diff.fields.push(field_diff);
    }

    if was_modified {
        Ok(Diff::Modified(DiffType::Tuple(diff)))
    } else {
        Ok(Diff::NoChange)
    }
}

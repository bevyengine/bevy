use crate::diff::{Diff, DiffError, DiffResult, DiffType};
use crate::{Reflect, ReflectKind, ReflectRef, Struct};
use bevy_utils::HashMap;
use std::fmt::{Debug, Formatter};

/// Diff object for (structs)[Struct].
#[derive(Clone)]
pub struct DiffedStruct<'old, 'new> {
    new_value: &'new dyn Struct,
    fields: HashMap<&'old str, Diff<'old, 'new>>,
    field_order: Vec<&'old str>,
}

impl<'old, 'new> DiffedStruct<'old, 'new> {
    /// Returns the "new" struct value.
    pub fn new_value(&self) -> &'new dyn Struct {
        self.new_value
    }

    /// Returns the [`Diff`] for the field with the given name.
    pub fn field(&self, name: &str) -> Option<&Diff<'old, 'new>> {
        self.fields.get(name)
    }

    /// Returns the [`Diff`] for the field at the given index.
    pub fn field_at(&self, index: usize) -> Option<&Diff<'old, 'new>> {
        self.field_order
            .get(index)
            .and_then(|name| self.fields.get(name))
    }

    /// Returns the number of fields in the struct.
    pub fn field_len(&self) -> usize {
        self.fields.len()
    }

    /// Returns an iterator over the name and [`Diff`] for _every_ field.
    pub fn field_iter(&self) -> impl Iterator<Item = (&'old str, &'_ Diff<'old, 'new>)> {
        self.field_order
            .iter()
            .copied()
            .map(|name| (name, self.fields.get(name).unwrap()))
    }
}

impl<'old, 'new> Debug for DiffedStruct<'old, 'new> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiffedStruct")
            .field("fields", &self.fields)
            .finish()
    }
}

/// Utility function for diffing two [`Struct`] objects.
pub fn diff_struct<'old, 'new, T: Struct>(
    old: &'old T,
    new: &'new dyn Reflect,
) -> DiffResult<'old, 'new> {
    let new = match new.reflect_ref() {
        ReflectRef::Struct(new) => new,
        new => {
            return Err(DiffError::KindMismatch {
                expected: ReflectKind::Struct,
                received: new.kind(),
            })
        }
    };

    let (old_info, new_info) = old
        .get_represented_type_info()
        .zip(new.get_represented_type_info())
        .ok_or(DiffError::MissingInfo)?;

    if old_info.type_id() != new_info.type_id() {
        return Ok(Diff::Replaced(new.as_reflect()));
    }

    let mut diff = DiffedStruct {
        new_value: new,
        fields: HashMap::with_capacity(new.field_len()),
        field_order: Vec::with_capacity(new.field_len()),
    };

    let mut was_modified = false;
    for (field_idx, old_field) in old.iter_fields().enumerate() {
        let field_name = old.name_at(field_idx).unwrap();
        let new_field = new.field(field_name).ok_or(DiffError::MissingField)?;
        let field_diff = old_field.diff(new_field)?;
        was_modified |= !matches!(field_diff, Diff::NoChange);
        diff.fields.insert(field_name, field_diff);
        diff.field_order.push(field_name);
    }

    if was_modified {
        Ok(Diff::Modified(DiffType::Struct(diff)))
    } else {
        Ok(Diff::NoChange)
    }
}

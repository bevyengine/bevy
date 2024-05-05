use crate::diff::{Diff, DiffError, DiffResult, DiffType, ValueDiff};
use crate::{Reflect, ReflectKind, ReflectRef, Struct, TypeInfo};
use bevy_utils::HashMap;
use std::borrow::Cow;
use std::fmt::{Debug, Formatter};

/// Diff object for (structs)[Struct].
pub struct StructDiff<'old, 'new> {
    type_info: &'static TypeInfo,
    fields: HashMap<Cow<'old, str>, Diff<'old, 'new>>,
    field_order: Vec<Cow<'old, str>>,
}

impl<'old, 'new> StructDiff<'old, 'new> {
    pub(crate) fn new(type_info: &'static TypeInfo, field_len: usize) -> Self {
        Self {
            type_info,
            fields: HashMap::with_capacity(field_len),
            field_order: Vec::with_capacity(field_len),
        }
    }

    /// Returns the [`TypeInfo`] of the reflected value currently being diffed.
    pub fn type_info(&self) -> &TypeInfo {
        self.type_info
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
    pub fn field_iter(&self) -> impl Iterator<Item = (&'_ str, &'_ Diff<'old, 'new>)> {
        self.field_order
            .iter()
            .map(|name| (name.as_ref(), self.fields.get(name).unwrap()))
    }

    pub(crate) fn push(&mut self, field_name: Cow<'old, str>, field_diff: Diff<'old, 'new>) {
        self.fields.insert(field_name.clone(), field_diff);
        self.field_order.push(field_name);
    }
}

impl<'old, 'new> Debug for StructDiff<'old, 'new> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StructDiff")
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
        return Ok(Diff::Replaced(ValueDiff::Borrowed(new.as_reflect())));
    }

    let mut diff = StructDiff::new(old_info, new.field_len());

    let mut was_modified = false;
    for (field_idx, old_field) in old.iter_fields().enumerate() {
        let field_name = old.name_at(field_idx).unwrap();
        let new_field = new.field(field_name).ok_or(DiffError::MissingField)?;
        let field_diff = old_field.diff(new_field)?;
        was_modified |= !matches!(field_diff, Diff::NoChange(_));
        diff.push(Cow::Borrowed(field_name), field_diff);
    }

    if was_modified {
        Ok(Diff::Modified(DiffType::Struct(diff)))
    } else {
        Ok(Diff::NoChange(old))
    }
}

use crate::diff::{Diff, DiffError, DiffResult, DiffType, ValueDiff};
use crate::{Map, Reflect, ReflectKind, ReflectRef, TypeInfo};
use std::fmt::{Debug, Formatter};
use std::slice::Iter;

/// Indicates the difference between two [`Map`] entries.
///
/// See the [module-level docs](crate::diff) for more details.
#[derive(Debug)]
pub enum EntryDiff<'old, 'new> {
    /// An entry with the given key was removed.
    Deleted(ValueDiff<'old>),
    /// An entry with the given key and value was added.
    Inserted(ValueDiff<'new>, ValueDiff<'new>),
    /// The entry with the given key was modified.
    Modified(ValueDiff<'old>, Diff<'old, 'new>),
}

/// Diff object for [maps](Map).
pub struct MapDiff<'old, 'new> {
    type_info: &'static TypeInfo,
    changes: Vec<EntryDiff<'old, 'new>>,
}

impl<'old, 'new> MapDiff<'old, 'new> {
    /// Returns the [`TypeInfo`] of the reflected value currently being diffed.
    pub fn type_info(&self) -> &TypeInfo {
        self.type_info
    }

    /// Returns the number of _changes_ made to the map.
    pub fn len_changes(&self) -> usize {
        self.changes.len()
    }

    /// Returns an iterator over the unordered sequence of edits needed to transform
    /// the "old" map into the "new" one.
    pub fn iter_changes(&self) -> Iter<'_, EntryDiff<'old, 'new>> {
        self.changes.iter()
    }
}

impl<'old, 'new> Debug for MapDiff<'old, 'new> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MapDiff")
            .field("changes", &self.changes)
            .finish()
    }
}

/// Utility function for diffing two [`Map`] objects.
pub fn diff_map<'old, 'new, T: Map>(
    old: &'old T,
    new: &'new dyn Reflect,
) -> DiffResult<'old, 'new> {
    let new = match new.reflect_ref() {
        ReflectRef::Map(new) => new,
        new => {
            return Err(DiffError::KindMismatch {
                expected: ReflectKind::Map,
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

    let mut diff = MapDiff::<'old, 'new> {
        type_info: old_info,
        changes: Vec::with_capacity(new.len()),
    };

    let mut was_modified = false;
    for (old_key, old_value) in old.iter() {
        if let Some(new_value) = new.get(old_key) {
            let value_diff = old_value.diff(new_value)?;
            if !matches!(value_diff, Diff::NoChange(_)) {
                was_modified = true;
                diff.changes.push(EntryDiff::Modified(
                    ValueDiff::Borrowed(old_key),
                    value_diff,
                ));
            }
        } else {
            was_modified = true;
            diff.changes
                .push(EntryDiff::Deleted(ValueDiff::Borrowed(old_key)));
        }
    }

    for (new_key, new_value) in new.iter() {
        if old.get(new_key).is_none() {
            was_modified = true;
            diff.changes.push(EntryDiff::Inserted(
                ValueDiff::Borrowed(new_key),
                ValueDiff::Borrowed(new_value),
            ));
        }
    }

    if was_modified {
        Ok(Diff::Modified(DiffType::Map(diff)))
    } else {
        Ok(Diff::NoChange(old))
    }
}

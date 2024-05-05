use crate::diff::{Diff, DiffError, DiffResult, DiffType};
use crate::{Map, Reflect, ReflectKind, ReflectRef};
use std::fmt::{Debug, Formatter};
use std::slice::Iter;

/// Indicates the difference between two [`Map`] entries.
///
/// See the [module-level docs](crate::diff) for more details.
#[derive(Clone, Debug)]
pub enum MapDiff<'old, 'new> {
    /// An entry with the given key was removed.
    Deleted(&'old dyn Reflect),
    /// An entry with the given key and value was added.
    Inserted(&'new dyn Reflect, &'new dyn Reflect),
    /// The entry with the given key was modified.
    Modified(&'old dyn Reflect, Diff<'old, 'new>),
}

/// Diff object for [maps](Map).
#[derive(Clone)]
pub struct DiffedMap<'old, 'new> {
    new_value: &'new dyn Map,
    changes: Vec<MapDiff<'old, 'new>>,
}

impl<'old, 'new> DiffedMap<'old, 'new> {
    /// Returns the "new" map value.
    pub fn new_value(&self) -> &'new dyn Map {
        self.new_value
    }

    /// Returns the number of _changes_ made to the map.
    pub fn len_changes(&self) -> usize {
        self.changes.len()
    }

    /// Returns an iterator over the unordered sequence of edits needed to transform
    /// the "old" map into the "new" one.
    pub fn iter_changes(&self) -> Iter<'_, MapDiff<'old, 'new>> {
        self.changes.iter()
    }
}

impl<'old, 'new> Debug for DiffedMap<'old, 'new> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiffedMap")
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
        return Ok(Diff::Replaced(new.as_reflect()));
    }

    let mut diff = DiffedMap::<'old, 'new> {
        new_value: new,
        changes: Vec::with_capacity(new.len()),
    };

    let mut was_modified = false;
    for (old_key, old_value) in old.iter() {
        if let Some(new_value) = new.get(old_key) {
            let value_diff = old_value.diff(new_value)?;
            if !matches!(value_diff, Diff::NoChange(_)) {
                was_modified = true;
                diff.changes.push(MapDiff::Modified(old_key, value_diff));
            }
        } else {
            was_modified = true;
            diff.changes.push(MapDiff::Deleted(old_key));
        }
    }

    for (new_key, new_value) in new.iter() {
        if matches!(old.get(new_key), None) {
            was_modified = true;
            diff.changes.push(MapDiff::Inserted(new_key, new_value));
        }
    }

    if was_modified {
        Ok(Diff::Modified(DiffType::Map(diff)))
    } else {
        Ok(Diff::NoChange(old))
    }
}

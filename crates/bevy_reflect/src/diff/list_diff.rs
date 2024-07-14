use crate::diff::{Diff, DiffError, DiffResult, DiffType, ValueDiff};
use crate::{List, Reflect, ReflectKind, ReflectRef, TypeInfo};
use std::fmt::{Debug, Formatter};
use std::slice::Iter;

/// Indicates the difference between two [`List`] elements.
///
/// See the [module-level docs](crate::diff) for more details.
#[derive(Debug)]
pub enum ElementDiff<'new> {
    /// The element at the given index was deleted.
    Deleted(usize),
    /// An element was inserted _before_ the given index.
    Inserted(usize, ValueDiff<'new>),
}

impl<'new> ElementDiff<'new> {
    pub fn index(&self) -> usize {
        match self {
            Self::Deleted(index) | Self::Inserted(index, _) => *index,
        }
    }

    pub fn clone_diff(&self) -> ElementDiff<'static> {
        match self {
            Self::Deleted(index) => ElementDiff::Deleted(*index),
            Self::Inserted(index, value_diff) => {
                ElementDiff::Inserted(*index, value_diff.clone_diff())
            }
        }
    }
}

/// Diff object for [lists](List).
pub struct ListDiff<'new> {
    type_info: &'static TypeInfo,
    changes: Vec<ElementDiff<'new>>,
    total_insertions: usize,
}

impl<'new> ListDiff<'new> {
    pub(crate) fn new(type_info: &'static TypeInfo, changes: Vec<ElementDiff<'new>>) -> Self {
        let total_insertions = changes
            .iter()
            .filter(|change| matches!(change, ElementDiff::Inserted(..)))
            .count();

        Self {
            type_info,
            changes,
            total_insertions,
        }
    }

    /// Returns the [`TypeInfo`] of the reflected value currently being diffed.
    pub fn type_info(&self) -> &TypeInfo {
        self.type_info
    }

    /// Returns the number of _changes_ made to the list.
    pub fn len_changes(&self) -> usize {
        self.changes.len()
    }

    /// Returns an iterator over the sequence of edits needed to transform
    /// the "old" list into the "new" one.
    pub fn iter_changes(&self) -> Iter<'_, ElementDiff<'new>> {
        self.changes.iter()
    }

    /// The total number of inserted elements.
    pub fn total_insertions(&self) -> usize {
        self.total_insertions
    }

    /// The total number of deleted elements.
    pub fn total_deletions(&self) -> usize {
        self.changes.len() - self.total_insertions
    }

    /// Take the changes contained in this diff.
    pub fn take_changes(self) -> Vec<ElementDiff<'new>> {
        self.changes
    }

    pub fn clone_diff(&self) -> ListDiff<'static> {
        ListDiff {
            type_info: self.type_info,
            changes: self.changes.iter().map(ElementDiff::clone_diff).collect(),
            total_insertions: self.total_insertions,
        }
    }
}

impl<'new> Debug for ListDiff<'new> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListDiff")
            .field("changes", &self.changes)
            .finish()
    }
}

/// Utility function for diffing two [`List`] objects.
pub fn diff_list<'old, 'new, T: List>(
    old: &'old T,
    new: &'new dyn Reflect,
) -> DiffResult<'old, 'new> {
    let new = match new.reflect_ref() {
        ReflectRef::List(new) => new,
        new => {
            return Err(DiffError::KindMismatch {
                expected: ReflectKind::List,
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

    let changes = ListDiffer::new(old, new).diff()?;

    if let Some(changes) = changes {
        Ok(Diff::Modified(DiffType::List(ListDiff::new(
            new_info, changes,
        ))))
    } else {
        Ok(Diff::NoChange)
    }
}

/// A helper struct for diffing two lists based on the [Myers Diffing Algorithm].
///
/// [Myers Diffing Algorithm]: http://www.xmailserver.org/diff2.pdf
struct ListDiffer<'old, 'new> {
    // AKA `a`.
    old: &'old dyn List,
    // AKA `b`.
    new: &'new dyn List,
    // AKA `MAX`.
    max_moves: i32,
    // AKA `V`.
    endpoints: Vec<i32>,
    snapshots: Vec<Vec<i32>>,
}

impl<'old, 'new> ListDiffer<'old, 'new> {
    pub fn new(old: &'old dyn List, new: &'new dyn List) -> Self {
        // Maximum Moves = Delete all of old + insert all of new
        let max_moves = old.len() + new.len();

        Self {
            old,
            new,
            max_moves: max_moves as i32,
            endpoints: vec![0; 2 * max_moves + 1],
            snapshots: Vec::with_capacity(max_moves),
        }
    }

    /// Perform the diff computation.
    ///
    /// Returns `None` if there was no change or `Some(changes)` if there was.
    pub fn diff(mut self) -> Result<Option<Vec<ElementDiff<'new>>>, DiffError> {
        if self.old.is_empty() && self.new.is_empty() {
            return Ok(None);
        }

        if self.old.is_empty() && !self.new.is_empty() {
            return Ok(Some(
                self.new
                    .iter()
                    .map(|value| ElementDiff::Inserted(0, ValueDiff::Borrowed(value)))
                    .collect(),
            ));
        }

        if !self.old.is_empty() && self.new.is_empty() {
            let mut vec = Vec::with_capacity(self.new.len());
            vec.fill_with(|| ElementDiff::Deleted(0));
            return Ok(Some(vec));
        }

        let ses = self.find_shortest_edit_script()?;
        if ses == 0 {
            Ok(None)
        } else {
            self.create_change_list(ses).map(Some)
        }
    }

    /// Uses the SES computation to create a list of changes to transform the [old] list to the [new] one.
    ///
    /// [old]: Self::old
    /// [new]: Self::new
    fn create_change_list(&self, ses: i32) -> Result<Vec<ElementDiff<'new>>, DiffError> {
        let mut x = self.old.len() as i32;
        let mut y = self.new.len() as i32;

        let mut changes = Vec::<ElementDiff<'new>>::with_capacity(ses as usize);

        // Start at end and work backwards to d = 0 (exclusive)
        for d in (1..=ses).rev() {
            let snapshot = &self.snapshots[d as usize - 1];

            let k = x - y;
            let x_insert = Self::get_wrapped(k + 1, snapshot);
            let x_delete = Self::get_wrapped(k - 1, snapshot);

            let (prev_x, prev_y, diff) = if k == -d || (k != d && x_insert > x_delete) {
                // Insertion was performed
                let prev_y = x_insert - (k + 1);
                let diff = ElementDiff::Inserted(
                    x_insert as usize,
                    ValueDiff::Borrowed(self.new_value(prev_y as usize)),
                );
                (x_insert, prev_y, diff)
            } else {
                // Deletion was performed
                let prev_y = x_delete - (k - 1);
                let diff = ElementDiff::Deleted(x_delete as usize);
                (x_delete, prev_y, diff)
            };

            (x, y) = (prev_x, prev_y);
            changes.push(diff);
        }

        // Out changes are in reverse order at this point, reverse them to the correct order
        changes.reverse();
        Ok(changes)
    }

    /// Finds the SES between the [old] and [new] lists.
    ///
    /// [old]: Self::old
    /// [new]: Self::new
    fn find_shortest_edit_script(&mut self) -> Result<i32, DiffError> {
        let max_moves = self.max_moves;
        let n = self.old.len() as i32;
        let m = self.new.len() as i32;

        for d in 0..=max_moves {
            for k in (-d..=d).step_by(2) {
                let x_insert = self.get_endpoint(k + 1);
                let x_delete = self.get_endpoint(k - 1);

                let mut x = if k == -d || (k != d && x_insert > x_delete) {
                    // Perform an insertion
                    x_insert
                } else {
                    // Perform a deletion
                    x_delete + 1
                };

                let mut y = x - k;

                while x < n
                    && y < m
                    && self.is_equal(self.old_value(x as usize), self.new_value(y as usize))?
                {
                    // Cross the diagonals (i.e. no change)
                    x += 1;
                    y += 1;
                }

                self.set_endpoint(k, x);

                if x >= n && y >= m {
                    // End reached - SES found!
                    return Ok(d);
                }
            }

            self.snapshots.push(self.endpoints.clone());
        }

        unreachable!("The list diffing algorithm should guarantee we find the SES");
    }

    fn is_equal(&self, a: &dyn Reflect, b: &dyn Reflect) -> Result<bool, DiffError> {
        a.reflect_partial_eq(b).ok_or(DiffError::Incomparable)
    }

    fn old_value(&self, index: usize) -> &'old dyn Reflect {
        self.old.get(index).unwrap()
    }

    fn new_value(&self, index: usize) -> &'new dyn Reflect {
        self.new.get(index).unwrap()
    }

    fn get_endpoint(&self, index: i32) -> i32 {
        Self::get_wrapped(index, &self.endpoints)
    }

    fn set_endpoint(&mut self, index: i32, value: i32) {
        Self::set_wrapped(index, value, &mut self.endpoints);
    }

    /// Gets an element from the given slice at a _wrapped_ index,
    /// where negative values are offset from the end of the slice.
    fn get_wrapped(index: i32, slice: &[i32]) -> i32 {
        if index >= 0 {
            slice[index as usize]
        } else {
            let len = slice.len() as i32;
            slice[(len + index) as usize]
        }
    }

    /// Sets an element from the given slice at a _wrapped_ index,
    /// where negative values are offset from the end of the slice.
    fn set_wrapped(index: i32, value: i32, slice: &mut [i32]) {
        if index >= 0 {
            slice[index as usize] = value;
        } else {
            let len = slice.len() as i32;
            slice[(len + index) as usize] = value;
        }
    }
}

use crate::storage::SparseSetIndex;
use bevy_utils::HashSet;
use core::fmt;
use fixedbitset::FixedBitSet;
use std::marker::PhantomData;

/// A wrapper struct to make Debug representations of [`FixedBitSet`] easier
/// to read, when used to store [`SparseSetIndex`].
///
/// Instead of the raw integer representation of the `FixedBitSet`, the list of
/// `T` valid for [`SparseSetIndex`] is shown.
///
/// Normal `FixedBitSet` `Debug` output:
/// ```text
/// read_and_writes: FixedBitSet { data: [ 160 ], length: 8 }
/// ```
///
/// Which, unless you are a computer, doesn't help much understand what's in
/// the set. With `FormattedBitSet`, we convert the present set entries into
/// what they stand for, it is much clearer what is going on:
/// ```text
/// read_and_writes: [ ComponentId(5), ComponentId(7) ]
/// ```
struct FormattedBitSet<'a, T: SparseSetIndex> {
    bit_set: &'a FixedBitSet,
    _marker: PhantomData<T>,
}
impl<'a, T: SparseSetIndex> FormattedBitSet<'a, T> {
    fn new(bit_set: &'a FixedBitSet) -> Self {
        Self {
            bit_set,
            _marker: PhantomData,
        }
    }
}
impl<'a, T: SparseSetIndex + fmt::Debug> fmt::Debug for FormattedBitSet<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.bit_set.ones().map(T::get_sparse_set_index))
            .finish()
    }
}

/// Tracks read and write access to specific elements in a collection.
///
/// Used internally to ensure soundness during system initialization and execution.
/// See the [`is_compatible`](Access::is_compatible) and [`get_conflicts`](Access::get_conflicts) functions.
#[derive(Clone, Eq, PartialEq)]
pub struct Access<T: SparseSetIndex> {
    /// All accessed elements.
    reads_and_writes: FixedBitSet,
    /// The exclusively-accessed elements.
    writes: FixedBitSet,
    /// Is `true` if this has access to all elements in the collection?
    /// This field is a performance optimization for `&World` (also harder to mess up for soundness).
    reads_all: bool,
    marker: PhantomData<T>,
}

impl<T: SparseSetIndex + fmt::Debug> fmt::Debug for Access<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Access")
            .field(
                "read_and_writes",
                &FormattedBitSet::<T>::new(&self.reads_and_writes),
            )
            .field("writes", &FormattedBitSet::<T>::new(&self.writes))
            .field("reads_all", &self.reads_all)
            .finish()
    }
}
impl<T: SparseSetIndex> Default for Access<T> {
    fn default() -> Self {
        Self {
            reads_all: false,
            reads_and_writes: Default::default(),
            writes: Default::default(),
            marker: PhantomData,
        }
    }
}

impl<T: SparseSetIndex> Access<T> {
    /// Increases the set capacity to the specified amount.
    ///
    /// Does nothing if `capacity` is less than or equal to the current value.
    pub fn grow(&mut self, capacity: usize) {
        self.reads_and_writes.grow(capacity);
        self.writes.grow(capacity);
    }

    /// Adds access to the element given by `index`.
    pub fn add_read(&mut self, index: T) {
        self.reads_and_writes.grow(index.sparse_set_index() + 1);
        self.reads_and_writes.insert(index.sparse_set_index());
    }

    /// Adds exclusive access to the element given by `index`.
    pub fn add_write(&mut self, index: T) {
        self.reads_and_writes.grow(index.sparse_set_index() + 1);
        self.reads_and_writes.insert(index.sparse_set_index());
        self.writes.grow(index.sparse_set_index() + 1);
        self.writes.insert(index.sparse_set_index());
    }

    /// Returns `true` if this can access the element given by `index`.
    pub fn has_read(&self, index: T) -> bool {
        self.reads_all || self.reads_and_writes.contains(index.sparse_set_index())
    }

    /// Returns `true` if this can exclusively access the element given by `index`.
    pub fn has_write(&self, index: T) -> bool {
        self.writes.contains(index.sparse_set_index())
    }

    /// Sets this as having access to all indexed elements (i.e. `&World`).
    pub fn read_all(&mut self) {
        self.reads_all = true;
    }

    /// Returns `true` if this has access to all indexed elements (i.e. `&World`).
    pub fn has_read_all(&self) -> bool {
        self.reads_all
    }

    /// Removes all accesses.
    pub fn clear(&mut self) {
        self.reads_all = false;
        self.reads_and_writes.clear();
        self.writes.clear();
    }

    /// Adds all access from `other`.
    pub fn extend(&mut self, other: &Access<T>) {
        self.reads_all = self.reads_all || other.reads_all;
        self.reads_and_writes.union_with(&other.reads_and_writes);
        self.writes.union_with(&other.writes);
    }

    /// Returns `true` if the access and `other` can be active at the same time.
    ///
    /// `Access` instances are incompatible if one can write
    /// an element that the other can read or write.
    pub fn is_compatible(&self, other: &Access<T>) -> bool {
        // Only systems that do not write data are compatible with systems that operate on `&World`.
        if self.reads_all {
            return other.writes.count_ones(..) == 0;
        }

        if other.reads_all {
            return self.writes.count_ones(..) == 0;
        }

        self.writes.is_disjoint(&other.reads_and_writes)
            && other.writes.is_disjoint(&self.reads_and_writes)
    }

    /// Returns a vector of elements that the access and `other` cannot access at the same time.
    pub fn get_conflicts(&self, other: &Access<T>) -> Vec<T> {
        let mut conflicts = FixedBitSet::default();
        if self.reads_all {
            conflicts.extend(other.writes.ones());
        }

        if other.reads_all {
            conflicts.extend(self.writes.ones());
        }
        conflicts.extend(self.writes.intersection(&other.reads_and_writes));
        conflicts.extend(self.reads_and_writes.intersection(&other.writes));
        conflicts
            .ones()
            .map(SparseSetIndex::get_sparse_set_index)
            .collect()
    }

    /// Returns the indices of the elements this has access to.
    pub fn reads_and_writes(&self) -> impl Iterator<Item = T> + '_ {
        self.reads_and_writes.ones().map(T::get_sparse_set_index)
    }

    /// Returns the indices of the elements this has non-exclusive access to.
    pub fn reads(&self) -> impl Iterator<Item = T> + '_ {
        self.reads_and_writes
            .difference(&self.writes)
            .map(T::get_sparse_set_index)
    }

    /// Returns the indices of the elements this has exclusive access to.
    pub fn writes(&self) -> impl Iterator<Item = T> + '_ {
        self.writes.ones().map(T::get_sparse_set_index)
    }
}

/// An [`Access`] that has been filtered to include and exclude certain combinations of elements.
///
/// Used internally to statically check if queries are disjoint.
///
/// Subtle: a `read` or `write` in `access` should not be considered to imply a
/// `with` access.
///
/// For example consider `Query<Option<&T>>` this only has a `read` of `T` as doing
/// otherwise would allow for queries to be considered disjoint when they shouldn't:
/// - `Query<(&mut T, Option<&U>)>` read/write `T`, read `U`, with `U`
/// - `Query<&mut T, Without<U>>` read/write `T`, without `U`
/// from this we could reasonably conclude that the queries are disjoint but they aren't.
///
/// In order to solve this the actual access that `Query<(&mut T, Option<&U>)>` has
/// is read/write `T`, read `U`. It must still have a read `U` access otherwise the following
/// queries would be incorrectly considered disjoint:
/// - `Query<&mut T>`  read/write `T`
/// - `Query<Option<&T>` accesses nothing
///
/// See comments the `WorldQuery` impls of `AnyOf`/`Option`/`Or` for more information.
#[derive(Clone, Eq, PartialEq)]
pub struct FilteredAccess<T: SparseSetIndex> {
    access: Access<T>,
    with: FixedBitSet,
    without: FixedBitSet,
}
impl<T: SparseSetIndex + fmt::Debug> fmt::Debug for FilteredAccess<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FilteredAccess")
            .field("access", &self.access)
            .field("with", &FormattedBitSet::<T>::new(&self.with))
            .field("without", &FormattedBitSet::<T>::new(&self.without))
            .finish()
    }
}

impl<T: SparseSetIndex> Default for FilteredAccess<T> {
    fn default() -> Self {
        Self {
            access: Access::default(),
            with: Default::default(),
            without: Default::default(),
        }
    }
}

impl<T: SparseSetIndex> From<FilteredAccess<T>> for FilteredAccessSet<T> {
    fn from(filtered_access: FilteredAccess<T>) -> Self {
        let mut base = FilteredAccessSet::<T>::default();
        base.add(filtered_access);
        base
    }
}

impl<T: SparseSetIndex> FilteredAccess<T> {
    /// Returns a reference to the underlying unfiltered access.
    #[inline]
    pub fn access(&self) -> &Access<T> {
        &self.access
    }

    /// Returns a mutable reference to the underlying unfiltered access.
    #[inline]
    pub fn access_mut(&mut self) -> &mut Access<T> {
        &mut self.access
    }

    /// Adds access to the element given by `index`.
    pub fn add_read(&mut self, index: T) {
        self.access.add_read(index.clone());
        self.add_with(index);
    }

    /// Adds exclusive access to the element given by `index`.
    pub fn add_write(&mut self, index: T) {
        self.access.add_write(index.clone());
        self.add_with(index);
    }

    /// Retains only combinations where the element given by `index` is also present.
    pub fn add_with(&mut self, index: T) {
        self.with.grow(index.sparse_set_index() + 1);
        self.with.insert(index.sparse_set_index());
    }

    /// Retains only combinations where the element given by `index` is not present.
    pub fn add_without(&mut self, index: T) {
        self.without.grow(index.sparse_set_index() + 1);
        self.without.insert(index.sparse_set_index());
    }

    pub fn extend_intersect_filter(&mut self, other: &FilteredAccess<T>) {
        self.without.intersect_with(&other.without);
        self.with.intersect_with(&other.with);
    }

    pub fn extend_access(&mut self, other: &FilteredAccess<T>) {
        self.access.extend(&other.access);
    }

    /// Returns `true` if this and `other` can be active at the same time.
    pub fn is_compatible(&self, other: &FilteredAccess<T>) -> bool {
        self.access.is_compatible(&other.access)
            || !self.with.is_disjoint(&other.without)
            || !other.with.is_disjoint(&self.without)
    }

    /// Returns a vector of elements that this and `other` cannot access at the same time.
    pub fn get_conflicts(&self, other: &FilteredAccess<T>) -> Vec<T> {
        if !self.is_compatible(other) {
            // filters are disjoint, so we can just look at the unfiltered intersection
            return self.access.get_conflicts(&other.access);
        }
        Vec::new()
    }

    /// Adds all access and filters from `other`.
    pub fn extend(&mut self, access: &FilteredAccess<T>) {
        self.access.extend(&access.access);
        self.with.union_with(&access.with);
        self.without.union_with(&access.without);
    }

    /// Sets the underlying unfiltered access as having access to all indexed elements.
    pub fn read_all(&mut self) {
        self.access.read_all();
    }
}

/// A collection of [`FilteredAccess`] instances.
///
/// Used internally to statically check if systems have conflicting access.
///
/// It stores multiple sets of accesses.
/// - A "combined" set, which is the access of all filters in this set combined.
/// - The set of access of each individual filters in this set.
#[derive(Debug, Clone)]
pub struct FilteredAccessSet<T: SparseSetIndex> {
    combined_access: Access<T>,
    filtered_accesses: Vec<FilteredAccess<T>>,
}

impl<T: SparseSetIndex> FilteredAccessSet<T> {
    /// Returns a reference to the unfiltered access of the entire set.
    #[inline]
    pub fn combined_access(&self) -> &Access<T> {
        &self.combined_access
    }

    /// Returns `true` if this and `other` can be active at the same time.
    ///
    /// Access conflict resolution happen in two steps:
    /// 1. A "coarse" check, if there is no mutual unfiltered conflict between
    ///    `self` and `other`, we already know that the two access sets are
    ///    compatible.
    /// 2. A "fine grained" check, it kicks in when the "coarse" check fails.
    ///    the two access sets might still be compatible if some of the accesses
    ///    are restricted with the `With` or `Without` filters so that access is
    ///    mutually exclusive. The fine grained phase iterates over all filters in
    ///    the `self` set and compares it to all the filters in the `other` set,
    ///    making sure they are all mutually compatible.
    pub fn is_compatible(&self, other: &FilteredAccessSet<T>) -> bool {
        if self.combined_access.is_compatible(other.combined_access()) {
            return true;
        }
        for filtered in &self.filtered_accesses {
            for other_filtered in &other.filtered_accesses {
                if !filtered.is_compatible(other_filtered) {
                    return false;
                }
            }
        }
        true
    }

    /// Returns a vector of elements that this set and `other` cannot access at the same time.
    pub fn get_conflicts(&self, other: &FilteredAccessSet<T>) -> Vec<T> {
        // if the unfiltered access is incompatible, must check each pair
        let mut conflicts = HashSet::new();
        if !self.combined_access.is_compatible(other.combined_access()) {
            for filtered in &self.filtered_accesses {
                for other_filtered in &other.filtered_accesses {
                    conflicts.extend(filtered.get_conflicts(other_filtered).into_iter());
                }
            }
        }
        conflicts.into_iter().collect()
    }

    /// Returns a vector of elements that this set and `other` cannot access at the same time.
    pub fn get_conflicts_single(&self, filtered_access: &FilteredAccess<T>) -> Vec<T> {
        // if the unfiltered access is incompatible, must check each pair
        let mut conflicts = HashSet::new();
        if !self.combined_access.is_compatible(filtered_access.access()) {
            for filtered in &self.filtered_accesses {
                conflicts.extend(filtered.get_conflicts(filtered_access).into_iter());
            }
        }
        conflicts.into_iter().collect()
    }

    /// Adds the filtered access to the set.
    pub fn add(&mut self, filtered_access: FilteredAccess<T>) {
        self.combined_access.extend(&filtered_access.access);
        self.filtered_accesses.push(filtered_access);
    }

    /// Adds a read access without filters to the set.
    pub(crate) fn add_unfiltered_read(&mut self, index: T) {
        let mut filter = FilteredAccess::default();
        filter.add_read(index);
        self.add(filter);
    }

    /// Adds a write access without filters to the set.
    pub(crate) fn add_unfiltered_write(&mut self, index: T) {
        let mut filter = FilteredAccess::default();
        filter.add_write(index);
        self.add(filter);
    }

    pub fn extend(&mut self, filtered_access_set: FilteredAccessSet<T>) {
        self.combined_access
            .extend(&filtered_access_set.combined_access);
        self.filtered_accesses
            .extend(filtered_access_set.filtered_accesses);
    }

    pub fn clear(&mut self) {
        self.combined_access.clear();
        self.filtered_accesses.clear();
    }
}

impl<T: SparseSetIndex> Default for FilteredAccessSet<T> {
    fn default() -> Self {
        Self {
            combined_access: Default::default(),
            filtered_accesses: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::query::{Access, FilteredAccess, FilteredAccessSet};

    #[test]
    fn read_all_access_conflicts() {
        // read_all / single write
        let mut access_a = Access::<usize>::default();
        access_a.grow(10);
        access_a.add_write(0);

        let mut access_b = Access::<usize>::default();
        access_b.read_all();

        assert!(!access_b.is_compatible(&access_a));

        // read_all / read_all
        let mut access_a = Access::<usize>::default();
        access_a.grow(10);
        access_a.read_all();

        let mut access_b = Access::<usize>::default();
        access_b.read_all();

        assert!(access_b.is_compatible(&access_a));
    }

    #[test]
    fn access_get_conflicts() {
        let mut access_a = Access::<usize>::default();
        access_a.add_read(0);
        access_a.add_read(1);

        let mut access_b = Access::<usize>::default();
        access_b.add_read(0);
        access_b.add_write(1);

        assert_eq!(access_a.get_conflicts(&access_b), vec![1]);

        let mut access_c = Access::<usize>::default();
        access_c.add_write(0);
        access_c.add_write(1);

        assert_eq!(access_a.get_conflicts(&access_c), vec![0, 1]);
        assert_eq!(access_b.get_conflicts(&access_c), vec![0, 1]);

        let mut access_d = Access::<usize>::default();
        access_d.add_read(0);

        assert_eq!(access_d.get_conflicts(&access_a), vec![]);
        assert_eq!(access_d.get_conflicts(&access_b), vec![]);
        assert_eq!(access_d.get_conflicts(&access_c), vec![0]);
    }

    #[test]
    fn filtered_combined_access() {
        let mut access_a = FilteredAccessSet::<usize>::default();
        access_a.add_unfiltered_read(1);

        let mut filter_b = FilteredAccess::<usize>::default();
        filter_b.add_write(1);

        let conflicts = access_a.get_conflicts_single(&filter_b);
        assert_eq!(
            &conflicts,
            &[1_usize],
            "access_a: {access_a:?}, filter_b: {filter_b:?}"
        );
    }

    #[test]
    fn filtered_access_extend() {
        let mut access_a = FilteredAccess::<usize>::default();
        access_a.add_read(0);
        access_a.add_read(1);
        access_a.add_with(2);

        let mut access_b = FilteredAccess::<usize>::default();
        access_b.add_read(0);
        access_b.add_write(3);
        access_b.add_without(4);

        access_a.extend(&access_b);

        let mut expected = FilteredAccess::<usize>::default();
        expected.add_read(0);
        expected.add_read(1);
        expected.add_with(2);
        expected.add_write(3);
        expected.add_without(4);

        assert!(access_a.eq(&expected));
    }
}

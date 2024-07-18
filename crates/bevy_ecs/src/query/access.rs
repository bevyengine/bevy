use crate::storage::SparseSetIndex;
use bevy_utils::HashSet;
use core::fmt;
use std::marker::PhantomData;
use smallvec::SmallVec;

const ACCESS_SMALL_VEC_SIZE: usize = 64;

/// Tracks read and write access to specific elements in a collection.
///
/// Used internally to ensure soundness during system initialization and execution.
/// See the [`is_compatible`](Access::is_compatible) and [`get_conflicts`](Access::get_conflicts) functions.
#[derive(Clone, Eq, PartialEq)]
pub struct Access<T> {
    /// All accessed elements.
    reads_and_writes: SortedSmallVec<ACCESS_SMALL_VEC_SIZE>,
    /// The exclusively-accessed elements.
    writes: SortedSmallVec<ACCESS_SMALL_VEC_SIZE>,
    /// Is `true` if this has access to all elements in the collection.
    /// This field is a performance optimization for `&World` (also harder to mess up for soundness).
    reads_all: bool,
    /// Is `true` if this has mutable access to all elements in the collection.
    /// If this is true, then `reads_all` must also be true.
    writes_all: bool,
    // Elements that are not accessed, but whose presence in an archetype affect query results.
    archetypal: SortedSmallVec<ACCESS_SMALL_VEC_SIZE>,
    marker: std::marker::PhantomData<T>,
}

impl<T: SparseSetIndex + fmt::Debug> fmt::Debug for Access<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Access")
            .field(
                "read_and_writes", &self.reads_and_writes,
            )
            .field("writes", &self.writes)
            .field("reads_all", &self.reads_all)
            .field("writes_all", &self.writes_all)
            .field("archetypal", &self.archetypal)
            .finish()
    }
}

impl<T: SparseSetIndex> Default for Access<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct SortedSmallVec<const N: usize>(SmallVec<[usize; N]>);


impl<const N: usize> IntoIterator for SortedSmallVec<N> {

    type Item = usize;
    type IntoIter = <SmallVec<[usize; N]> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<const N: usize> SortedSmallVec<N> {
    fn new() -> Self {
        Self(SmallVec::new())
    }

    const fn new_const() -> Self {
        Self(SmallVec::new_const())
    }

    pub(crate) fn ones(&self) -> impl Iterator<Item = usize> + '_ {
        self.0.iter().copied()
    }

    /// Insert the value if it's not already present in the vector.
    /// Maintains a sorted order.
    fn insert(&mut self, index: usize) {
        match self.0.binary_search(&index) {
            // element already present in the vector
            Ok(_) => {}
            Err(pos) => {
                self.0.insert(pos, index);
            }
        }
    }

    /// Returns true if the vector contains the value.
    fn contains(&self, index: usize) -> bool {
        self.0.binary_search(&index).is_ok()
    }

    /// Returns true if the vector is empty.
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Empties the contents of the vector
    pub(crate) fn clear(&mut self) {
        self.0.clear()
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    /// Adds all the elements from `other` into this vector. (skipping duplicates)
    fn union_with(&mut self, other: &Self) {
        let mut i = 0;
        let mut j = 0;
        while i < self.len() && j < other.len() {
            if self.0[i] < other.0[j] {
                i += 1;
            } else if self.0[i] > other.0[j] {
                self.0.insert(i, other.0[j]);
                j += 1;
            } else {
                i += 1;
                j += 1;
            }
        }
        while j < other.len() {
            self.0.push(other.0[j]);
            j += 1;
        }
    }

    /// Returns the elements that are in both `self` and `other`.
    fn intersection<'a>(&'a self, other: &'a Self) -> Intersection<'a, N> {
        Intersection {
            this: self,
            other,
            i: 0,
            j: 0,
        }
        // let mut i = 0;
        // let mut j = 0;
        // if other.len() < self.len() {
        //     return other.intersection(self);
        // }
        // let mut new = Self::new();
        // while i < self.len() && j < other.len() {
        //     if (i == 0 || self.0[i] != self.0[i - 1]) && self.0[i] == other.0[j] {
        //         new.0.push(self.0[i]);
        //         i += 1;
        //         j += 1;
        //     } else if self.0[i] < other.0[j] {
        //         i += 1;
        //     } else {
        //         j += 1;
        //     }
        // }
        // new
    }

    /// Return the elements that are in `self` but not in `other`.
    fn difference<'a>(&'a self, other: &'a Self) -> Difference<'a, N> {
        Difference {
            this: self,
            other,
            i: 0,
            j: 0,
        }
        // let mut i = 0;
        // let mut j = 0;
        // let mut new = Self::new();
        // while i < self.len() && j < other.len() {
        //     if self.0[i] == other.0[j] {
        //         i += 1;
        //         j += 1;
        //     } else if (i == 0 || self.0[i] != self.0[i - 1]) && self.0[i] < other.0[j] {
        //         new.0.push(self.0[i]);
        //         i += 1;
        //     } else {
        //         j += 1;
        //     }
        // }
        // while i < self.len() {
        //     if i == 0 || self.0[i] != self.0[i - 1] {
        //         new.0.push(self.0[i]);
        //     }
        //     i += 1;
        // }
        // new
    }

    /// Returns true if the two vectors have no common elements.
    fn is_disjoint(&self, other: &Self) -> bool {
        self.intersection(other).next().is_none()
    }

    /// Returns true if all the elements in `self` are also in `other`.
    fn is_subset(&self, other: &Self) -> bool {
        self.difference(other).next().is_none()
    }
}

impl<const N: usize> Extend<usize> for SortedSmallVec<N> {
    fn extend<T: IntoIterator<Item=usize>>(&mut self, other: T) {
        let mut i = 0;
        let mut other_iter = other.into_iter();
        let mut other_val = other_iter.next();
        while i < self.len() && other_val.is_some() {
            let val_j = other_val.unwrap();
            if self.0[i] < val_j {
                i += 1;
            } else if self.0[i] > val_j {
                self.0.insert(i, val_j);
                other_val = other_iter.next();
            } else {
                i += 1;
                other_val = other_iter.next();
            }
        }
        while let Some(val_j) = other_val {
            self.0.push(val_j);
            other_val = other_iter.next();
        }
    }
}

/// Intersection between `this` and `other` sorted vectors.
struct Intersection<'a, const N: usize> {
    this: &'a SortedSmallVec<N>,
    other: &'a SortedSmallVec<N>,
    i: usize,
    j: usize,
}

impl<'a, const N: usize> Iterator for Intersection<'a, N> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let mut res = None;
        while self.i < self.this.len() && self.j < self.other.len() {
            if (self.i == 0 || self.this.0[self.i] != self.this.0[self.i - 1]) && self.this.0[self.i] == self.other.0[self.j] {
                res = Some(self.this.0[self.i]);
                self.i += 1;
                self.j += 1;
                return res;
            } else if self.this.0[self.i] < self.other.0[self.j] {
                self.i += 1;
            } else {
                self.j += 1;
            }
        }
        res
    }
}

/// Difference between `this` and `other` sorted vectors.
struct Difference<'a, const N: usize> {
    this: &'a SortedSmallVec<N>,
    other: &'a SortedSmallVec<N>,
    i: usize,
    j: usize,
}

impl<'a, const N: usize> Iterator for Difference<'a, N> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let mut res = None;
        while self.i < self.this.len() && self.j < self.other.len() {
            if self.this.0[self.i] == self.other.0[self.j] {
                self.i += 1;
                self.j += 1;
            } else if (self.i == 0 || self.this.0[self.i] != self.this.0[self.i - 1]) && self.this.0[self.i] < self.other.0[self.j] {
                res = Some(self.this.0[self.i]);
                self.i += 1;
                return res;
            } else {
                self.j += 1;
            }
        }
        if self.i < self.this.len() {
            if self.i == 0 || self.this.0[self.i] != self.this.0[self.i - 1] {
                res = Some(self.this.0[self.i]);
            }
            self.i += 1;
        }
        return res;
    }
}

impl<T: SparseSetIndex> Access<T> {
    /// Creates an empty [`Access`] collection.
    pub const fn new() -> Self {
        Self {
            reads_all: false,
            writes_all: false,
            reads_and_writes: SortedSmallVec::new_const(),
            writes: SortedSmallVec::new_const(),
            archetypal: SortedSmallVec::new_const(),
            marker: PhantomData,
        }
    }

    /// Adds access to the element given by `index`.
    pub fn add_read(&mut self, index: T) {
        self.reads_and_writes.insert(index.sparse_set_index())
    }

    /// Adds exclusive access to the element given by `index`.
    pub fn add_write(&mut self, index: T) {
        self.reads_and_writes.insert(index.sparse_set_index());
        self.writes.insert(index.sparse_set_index());
    }

    /// Adds an archetypal (indirect) access to the element given by `index`.
    ///
    /// This is for elements whose values are not accessed (and thus will never cause conflicts),
    /// but whose presence in an archetype may affect query results.
    ///
    /// Currently, this is only used for [`Has<T>`].
    ///
    /// [`Has<T>`]: crate::query::Has
    pub fn add_archetypal(&mut self, index: T) {
        self.archetypal.insert(index.sparse_set_index());
    }

    /// Returns `true` if this can access the element given by `index`.
    pub fn has_read(&self, index: T) -> bool {
        self.reads_all || self.reads_and_writes.contains(index.sparse_set_index())
    }

    /// Returns `true` if this can access anything.
    pub fn has_any_read(&self) -> bool {
        self.reads_all || !self.reads_and_writes.is_empty()
    }

    /// Returns `true` if this can exclusively access the element given by `index`.
    pub fn has_write(&self, index: T) -> bool {
        self.writes_all || self.writes.contains(index.sparse_set_index())
    }

    /// Returns `true` if this accesses anything mutably.
    pub fn has_any_write(&self) -> bool {
        self.writes_all || !self.writes.is_empty()
    }

    /// Returns true if this has an archetypal (indirect) access to the element given by `index`.
    ///
    /// This is an element whose value is not accessed (and thus will never cause conflicts),
    /// but whose presence in an archetype may affect query results.
    ///
    /// Currently, this is only used for [`Has<T>`].
    ///
    /// [`Has<T>`]: crate::query::Has
    pub fn has_archetypal(&self, index: T) -> bool {
        self.archetypal.contains(index.sparse_set_index())
    }

    /// Sets this as having access to all indexed elements (i.e. `&World`).
    pub fn read_all(&mut self) {
        self.reads_all = true;
    }

    /// Sets this as having mutable access to all indexed elements (i.e. `EntityMut`).
    pub fn write_all(&mut self) {
        self.reads_all = true;
        self.writes_all = true;
    }

    /// Returns `true` if this has access to all indexed elements (i.e. `&World`).
    pub fn has_read_all(&self) -> bool {
        self.reads_all
    }

    /// Returns `true` if this has write access to all indexed elements (i.e. `EntityMut`).
    pub fn has_write_all(&self) -> bool {
        self.writes_all
    }

    /// Removes all writes.
    pub fn clear_writes(&mut self) {
        self.writes_all = false;
        self.writes.clear();
    }

    /// Removes all accesses.
    pub fn clear(&mut self) {
        self.reads_all = false;
        self.writes_all = false;
        self.reads_and_writes.clear();
        self.writes.clear();
    }

    /// Adds all access from `other`.
    pub fn extend(&mut self, other: &Access<T>) {
        self.reads_all = self.reads_all || other.reads_all;
        self.writes_all = self.writes_all || other.writes_all;
        self.reads_and_writes.union_with(&other.reads_and_writes);
        self.writes.union_with(&other.writes);
    }

    /// Returns `true` if the access and `other` can be active at the same time.
    ///
    /// [`Access`] instances are incompatible if one can write
    /// an element that the other can read or write.
    pub fn is_compatible(&self, other: &Access<T>) -> bool {
        if self.writes_all {
            return !other.has_any_read();
        }

        if other.writes_all {
            return !self.has_any_read();
        }

        if self.reads_all {
            return !other.has_any_write();
        }

        if other.reads_all {
            return !self.has_any_write();
        }

        self.writes.is_disjoint(&other.reads_and_writes)
            && other.writes.is_disjoint(&self.reads_and_writes)
    }

    /// Returns `true` if the set is a subset of another, i.e. `other` contains
    /// at least all the values in `self`.
    pub fn is_subset(&self, other: &Access<T>) -> bool {
        if self.writes_all {
            return other.writes_all;
        }

        if other.writes_all {
            return true;
        }

        if self.reads_all {
            return other.reads_all;
        }

        if other.reads_all {
            return self.writes.is_subset(&other.writes);
        }

        self.reads_and_writes.is_subset(&other.reads_and_writes)
            && self.writes.is_subset(&other.writes)
    }

    /// Returns a vector of elements that the access and `other` cannot access at the same time.
    pub fn get_conflicts(&self, other: &Access<T>) -> Vec<T> {
        let mut conflicts = SortedSmallVec::<ACCESS_SMALL_VEC_SIZE>::new();
        if self.reads_all {
            // QUESTION: How to handle `other.writes_all`?
            conflicts.union_with(&other.writes);
        }

        if other.reads_all {
            // QUESTION: How to handle `self.writes_all`.
            conflicts.union_with(&self.writes);
        }

        if self.writes_all {
            conflicts.union_with(&other.reads_and_writes);
        }

        if other.writes_all {
            conflicts.union_with(&self.reads_and_writes);
        }

        conflicts.extend(self.writes.intersection(&other.reads_and_writes));
        conflicts.extend(self.reads_and_writes.intersection(&other.writes));
        conflicts.ones()
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

    /// Returns the indices of the elements that this has an archetypal access to.
    ///
    /// These are elements whose values are not accessed (and thus will never cause conflicts),
    /// but whose presence in an archetype may affect query results.
    ///
    /// Currently, this is only used for [`Has<T>`].
    ///
    /// [`Has<T>`]: crate::query::Has
    pub fn archetypal(&self) -> impl Iterator<Item = T> + '_ {
        self.archetypal.ones().map(T::get_sparse_set_index)
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
///     from this we could reasonably conclude that the queries are disjoint but they aren't.
///
/// In order to solve this the actual access that `Query<(&mut T, Option<&U>)>` has
/// is read/write `T`, read `U`. It must still have a read `U` access otherwise the following
/// queries would be incorrectly considered disjoint:
/// - `Query<&mut T>`  read/write `T`
/// - `Query<Option<&T>>` accesses nothing
///
/// See comments the [`WorldQuery`](super::WorldQuery) impls of [`AnyOf`](super::AnyOf)/`Option`/[`Or`](super::Or) for more information.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FilteredAccess<T: SparseSetIndex> {
    pub(crate) access: Access<T>,
    pub(crate) required: SortedSmallVec<ACCESS_SMALL_VEC_SIZE>,
    // An array of filter sets to express `With` or `Without` clauses in disjunctive normal form, for example: `Or<(With<A>, With<B>)>`.
    // Filters like `(With<A>, Or<(With<B>, Without<C>)>` are expanded into `Or<((With<A>, With<B>), (With<A>, Without<C>))>`.
    pub(crate) filter_sets: Vec<AccessFilters<T>>,
}

impl<T: SparseSetIndex> Default for FilteredAccess<T> {
    fn default() -> Self {
        Self {
            access: Access::default(),
            required: SortedSmallVec::new(),
            filter_sets: vec![AccessFilters::default()]
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
        self.add_required(index.clone());
        self.and_with(index);
    }

    /// Adds exclusive access to the element given by `index`.
    pub fn add_write(&mut self, index: T) {
        self.access.add_write(index.clone());
        self.add_required(index.clone());
        self.and_with(index);
    }

    fn add_required(&mut self, index: T) {
        self.required.insert(index.sparse_set_index());
    }

    /// Adds a `With` filter: corresponds to a conjunction (AND) operation.
    ///
    /// Suppose we begin with `Or<(With<A>, With<B>)>`, which is represented by an array of two `AccessFilter` instances.
    /// Adding `AND With<C>` via this method transforms it into the equivalent of  `Or<((With<A>, With<C>), (With<B>, With<C>))>`.
    pub fn and_with(&mut self, index: T) {
        for filter in &mut self.filter_sets {
            filter.with.insert(index.sparse_set_index());
        }
    }

    /// Adds a `Without` filter: corresponds to a conjunction (AND) operation.
    ///
    /// Suppose we begin with `Or<(With<A>, With<B>)>`, which is represented by an array of two `AccessFilter` instances.
    /// Adding `AND Without<C>` via this method transforms it into the equivalent of  `Or<((With<A>, Without<C>), (With<B>, Without<C>))>`.
    pub fn and_without(&mut self, index: T) {
        for filter in &mut self.filter_sets {
            filter.without.insert(index.sparse_set_index());
        }
    }

    /// Appends an array of filters: corresponds to a disjunction (OR) operation.
    ///
    /// As the underlying array of filters represents a disjunction,
    /// where each element (`AccessFilters`) represents a conjunction,
    /// we can simply append to the array.
    pub fn append_or(&mut self, other: &FilteredAccess<T>) {
        self.filter_sets.append(&mut other.filter_sets.clone());
    }

    /// Adds all of the accesses from `other` to `self`.
    pub fn extend_access(&mut self, other: &FilteredAccess<T>) {
        self.access.extend(&other.access);
    }

    /// Returns `true` if this and `other` can be active at the same time.
    pub fn is_compatible(&self, other: &FilteredAccess<T>) -> bool {
        if self.access.is_compatible(&other.access) {
            return true;
        }

        // If the access instances are incompatible, we want to check that whether filters can
        // guarantee that queries are disjoint.
        // Since the `filter_sets` array represents a Disjunctive Normal Form formula ("ORs of ANDs"),
        // we need to make sure that each filter set (ANDs) rule out every filter set from the `other` instance.
        //
        // For example, `Query<&mut C, Or<(With<A>, Without<B>)>>` is compatible `Query<&mut C, (With<B>, Without<A>)>`,
        // but `Query<&mut C, Or<(Without<A>, Without<B>)>>` isn't compatible with `Query<&mut C, Or<(With<A>, With<B>)>>`.
        self.filter_sets.iter().all(|filter| {
            other
                .filter_sets
                .iter()
                .all(|other_filter| filter.is_ruled_out_by(other_filter))
        })
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
    ///
    /// Corresponds to a conjunction operation (AND) for filters.
    ///
    /// Extending `Or<(With<A>, Without<B>)>` with `Or<(With<C>, Without<D>)>` will result in
    /// `Or<((With<A>, With<C>), (With<A>, Without<D>), (Without<B>, With<C>), (Without<B>, Without<D>))>`.
    pub fn extend(&mut self, other: &FilteredAccess<T>) {
        self.access.extend(&other.access);
        self.required.union_with(&other.required);

        // We can avoid allocating a new array of bitsets if `other` contains just a single set of filters:
        // in this case we can short-circuit by performing an in-place union for each bitset.
        if other.filter_sets.len() == 1 {
            for filter in &mut self.filter_sets {
                filter.with.union_with(&other.filter_sets[0].with);
                filter.without.union_with(&other.filter_sets[0].without);
            }
            return;
        }

        let mut new_filters = Vec::with_capacity(self.filter_sets.len() * other.filter_sets.len());
        for filter in &self.filter_sets {
            for other_filter in &other.filter_sets {
                let mut new_filter = filter.clone();
                new_filter.with.union_with(&other_filter.with);
                new_filter.without.union_with(&other_filter.without);
                new_filters.push(new_filter);
            }
        }
        self.filter_sets = new_filters;
    }

    /// Sets the underlying unfiltered access as having access to all indexed elements.
    pub fn read_all(&mut self) {
        self.access.read_all();
    }

    /// Sets the underlying unfiltered access as having mutable access to all indexed elements.
    pub fn write_all(&mut self) {
        self.access.write_all();
    }

    /// Returns `true` if the set is a subset of another, i.e. `other` contains
    /// at least all the values in `self`.
    pub fn is_subset(&self, other: &FilteredAccess<T>) -> bool {
        self.required.is_subset(&other.required) && self.access().is_subset(other.access())
    }

    /// Returns the indices of the elements that this access filters for.
    pub fn with_filters(&self) -> impl Iterator<Item = T> + '_ {
        self.filter_sets
            .iter()
            .flat_map(|f| f.with.ones().map(T::get_sparse_set_index))
    }

    /// Returns the indices of the elements that this access filters out.
    pub fn without_filters(&self) -> impl Iterator<Item = T> + '_ {
        self.filter_sets
            .iter()
            .flat_map(|f| f.without.ones().map(T::get_sparse_set_index))
    }
}

#[derive(Clone, Eq, PartialEq)]
pub(crate) struct AccessFilters<T> {
    pub(crate) with: SortedSmallVec<ACCESS_SMALL_VEC_SIZE>,
    pub(crate) without: SortedSmallVec<ACCESS_SMALL_VEC_SIZE>,
    _index_type: PhantomData<T>,
}

impl<T: SparseSetIndex + fmt::Debug> fmt::Debug for AccessFilters<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccessFilters")
            .field("with", &self.with)
            .field("without", &self.without)
            .finish()
    }
}

impl<T: SparseSetIndex> Default for AccessFilters<T> {
    fn default() -> Self {
        Self {
            with: SortedSmallVec::new(),
            without: SortedSmallVec::new(),
            _index_type: PhantomData,
        }
    }
}

impl<T: SparseSetIndex> AccessFilters<T> {
    fn is_ruled_out_by(&self, other: &Self) -> bool {
        // Although not technically complete, we don't consider the case when `AccessFilters`'s
        // `without` bitset contradicts its own `with` bitset (e.g. `(With<A>, Without<A>)`).
        // Such query would be considered compatible with any other query, but as it's almost
        // always an error, we ignore this case instead of treating such query as compatible
        // with others.
        !self.with.is_disjoint(&other.without) || !self.without.is_disjoint(&other.with)
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
    ///    are restricted with the [`With`](super::With) or [`Without`](super::Without) filters so that access is
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

    /// Adds all of the accesses from the passed set to `self`.
    pub fn extend(&mut self, filtered_access_set: FilteredAccessSet<T>) {
        self.combined_access
            .extend(&filtered_access_set.combined_access);
        self.filtered_accesses
            .extend(filtered_access_set.filtered_accesses);
    }

    /// Marks the set as reading all possible indices of type T.
    pub fn read_all(&mut self) {
        self.combined_access.read_all();
    }

    /// Marks the set as writing all T.
    pub fn write_all(&mut self) {
        self.combined_access.write_all();
    }

    /// Removes all accesses stored in this set.
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
    use crate::query::access::{AccessFilters, SortedSmallVec};
    use crate::query::{Access, FilteredAccess, FilteredAccessSet};
    use std::marker::PhantomData;
    use smallvec::{SmallVec, smallvec};

    #[test]
    fn sorted_vec_union() {
        let mut a = SortedSmallVec(SmallVec::from([1, 3, 4]));
        let b = SortedSmallVec(SmallVec::from([2, 3, 5]));
        a.union_with(&b);
        assert_eq!(a.0, SmallVec::from([1, 2, 3, 4, 5]));

        let mut a = SortedSmallVec(SmallVec::from([2, 3, 4]));
        let b = SortedSmallVec(SmallVec::from([1, 4, 5]));
        a.union_with(&b);
        assert_eq!(a.0, SmallVec::from([1, 2, 3, 4, 5]));
    }

    #[test]
    fn sorted_vec_intersection() {
        let a = SortedSmallVec(SmallVec::from([1, 3, 4, 6]));
        let b = SortedSmallVec(smallvec![7, 8]);
        assert_eq!(a.intersection(&b).collect::<Vec<_>>(), vec![]);

        let a = SortedSmallVec(SmallVec::from([2, 3, 4]));
        let b = SortedSmallVec(SmallVec::from([1, 3, 5]));
        assert_eq!(a.intersection(&b).collect::<Vec<_>>(), vec![3]);
    }

    #[test]
    fn sorted_vec_difference() {
        let a = SortedSmallVec(SmallVec::from([1, 3, 4, 6]));
        let b = SortedSmallVec(smallvec![7, 8]);
        assert_eq!(a.difference(&b).collect::<Vec<_>>(), vec![1, 3, 4, 6]);

        let a = SortedSmallVec(SmallVec::from([2, 3, 4]));
        let b = SortedSmallVec(SmallVec::from([1, 3, 5]));
        assert_eq!(a.difference(&b).collect::<Vec<_>>(), vec![2, 4]);
    }

    #[test]
    fn sorted_vec_is_subset() {
        let a = SortedSmallVec(SmallVec::from([1, 3, 4, 6]));
        let b = SortedSmallVec(smallvec![1, 3, 4, 6, 8]);
        assert_eq!(a.difference(&b).collect::<Vec<_>>(), vec![]);
        assert_eq!(a.is_subset(&b), true);

        // TODO: allow different values of N?
        let a = SortedSmallVec(SmallVec::from([2, 3, 4, 5]));
        let b = SortedSmallVec(SmallVec::from([1, 2, 3, 4]));
        assert_eq!(a.difference(&b).collect::<Vec<_>>(), vec![5]);
        assert_eq!(a.is_subset(&b), false);
    }

    #[test]
    fn read_all_access_conflicts() {
        // read_all / single write
        let mut access_a = Access::<usize>::default();
        access_a.add_write(0);

        let mut access_b = Access::<usize>::default();
        access_b.read_all();

        assert!(!access_b.is_compatible(&access_a));

        // read_all / read_all
        let mut access_a = Access::<usize>::default();
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
        access_a.and_with(2);

        let mut access_b = FilteredAccess::<usize>::default();
        access_b.add_read(0);
        access_b.add_write(3);
        access_b.and_without(4);

        access_a.extend(&access_b);

        let mut expected = FilteredAccess::<usize>::default();
        expected.add_read(0);
        expected.add_read(1);
        expected.and_with(2);
        expected.add_write(3);
        expected.and_without(4);

        assert!(access_a.eq(&expected));
    }

    #[test]
    fn filtered_access_extend_or() {
        let mut access_a = FilteredAccess::<usize>::default();
        // Exclusive access to `(&mut A, &mut B)`.
        access_a.add_write(0);
        access_a.add_write(1);

        // Filter by `With<C>`.
        let mut access_b = FilteredAccess::<usize>::default();
        access_b.and_with(2);

        // Filter by `(With<D>, Without<E>)`.
        let mut access_c = FilteredAccess::<usize>::default();
        access_c.and_with(3);
        access_c.and_without(4);

        // Turns `access_b` into `Or<(With<C>, (With<D>, Without<E>))>`.
        access_b.append_or(&access_c);
        // Applies the filters to the initial query, which corresponds to the FilteredAccess'
        // representation of `Query<(&mut A, &mut B), Or<(With<C>, (With<D>, Without<E>))>>`.
        access_a.extend(&access_b);

        // Construct the expected `FilteredAccess` struct.
        // The intention here is to test that exclusive access implied by `add_write`
        // forms correct normalized access structs when extended with `Or` filters.
        let mut expected = FilteredAccess::<usize>::default();
        expected.add_write(0);
        expected.add_write(1);
        // The resulted access is expected to represent `Or<((With<A>, With<B>, With<C>), (With<A>, With<B>, With<D>, Without<E>))>`.
        expected.filter_sets = vec![
            AccessFilters {
                with: SortedSmallVec(SmallVec::from(vec![0, 1, 2])),
                without: SortedSmallVec::new(),
                _index_type: PhantomData,
            },
            AccessFilters {
                with: SortedSmallVec(SmallVec::from(vec![0, 1, 3])),
                without: SortedSmallVec(SmallVec::from(vec![4])),
                _index_type: PhantomData,
            },
        ];

        assert_eq!(access_a, expected);
    }
}

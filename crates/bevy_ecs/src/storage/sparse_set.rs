use crate::{
    change_detection::MaybeLocation,
    component::{ComponentId, ComponentInfo, ComponentTicks, Tick, TickCells},
    entity::{Entity, EntityGeneration},
};
use alloc::{boxed::Box, vec::Vec};
use bevy_ptr::{OwningPtr, Ptr};
use core::{cell::UnsafeCell, hash::Hash, marker::PhantomData, num::NonZeroUsize, panic::Location};
use nonmax::{NonMaxU32, NonMaxUsize};

use super::ThinColumn;

#[derive(Debug)]
pub(crate) struct SparseArray<I, V = I> {
    values: Vec<Option<V>>,
    marker: PhantomData<I>,
}

/// A space-optimized version of [`SparseArray`] that cannot be changed
/// after construction.
#[derive(Debug)]
pub(crate) struct ImmutableSparseArray<I, V = I> {
    values: Box<[Option<V>]>,
    marker: PhantomData<I>,
}

impl<I: SparseSetIndex, V> Default for SparseArray<I, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<I, V> SparseArray<I, V> {
    #[inline]
    pub const fn new() -> Self {
        Self {
            values: Vec::new(),
            marker: PhantomData,
        }
    }
}

macro_rules! impl_sparse_array {
    ($ty:ident) => {
        impl<I: SparseSetIndex, V> $ty<I, V> {
            /// Returns `true` if the collection contains a value for the specified `index`.
            #[inline]
            pub fn contains(&self, index: I) -> bool {
                let index = index.sparse_set_index();
                self.values.get(index).is_some_and(Option::is_some)
            }

            /// Returns a reference to the value at `index`.
            ///
            /// Returns `None` if `index` does not have a value or if `index` is out of bounds.
            #[inline]
            pub fn get(&self, index: I) -> Option<&V> {
                let index = index.sparse_set_index();
                self.values.get(index).and_then(Option::as_ref)
            }
        }
    };
}

impl_sparse_array!(SparseArray);
impl_sparse_array!(ImmutableSparseArray);

impl<I: SparseSetIndex, V> SparseArray<I, V> {
    /// Inserts `value` at `index` in the array.
    ///
    /// If `index` is out-of-bounds, this will enlarge the buffer to accommodate it.
    #[inline]
    pub fn insert(&mut self, index: I, value: V) {
        let index = index.sparse_set_index();
        if index >= self.values.len() {
            self.values.resize_with(index + 1, || None);
        }
        self.values[index] = Some(value);
    }

    /// Returns a mutable reference to the value at `index`.
    ///
    /// Returns `None` if `index` does not have a value or if `index` is out of bounds.
    #[inline]
    pub fn get_mut(&mut self, index: I) -> Option<&mut V> {
        let index = index.sparse_set_index();
        self.values.get_mut(index).and_then(Option::as_mut)
    }

    /// Removes and returns the value stored at `index`.
    ///
    /// Returns `None` if `index` did not have a value or if `index` is out of bounds.
    #[inline]
    pub fn remove(&mut self, index: I) -> Option<V> {
        let index = index.sparse_set_index();
        self.values.get_mut(index).and_then(Option::take)
    }

    /// Removes all of the values stored within.
    pub fn clear(&mut self) {
        self.values.clear();
    }

    /// Converts the [`SparseArray`] into an immutable variant.
    pub(crate) fn into_immutable(self) -> ImmutableSparseArray<I, V> {
        ImmutableSparseArray {
            values: self.values.into_boxed_slice(),
            marker: PhantomData,
        }
    }
}

/// Represents the row of an [`Entity`] within a [`ComponentSparseSet`].
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
// SAFETY: Must be repr(transparent) due to the safety requirements on EntityLocation
#[repr(transparent)]
pub struct SparseSetRow(NonMaxU32);

impl SparseSetRow {
    /// Creates a `SparseSetRow`.
    #[inline]
    pub const fn new(index: NonMaxU32) -> Self {
        Self(index)
    }

    /// Gets the index of the row.
    #[inline]
    pub const fn index(self) -> usize {
        self.0.get() as usize
    }

    /// Gets the index of the row.
    #[inline]
    pub const fn index_u32(self) -> u32 {
        self.0.get()
    }
}

/// A sparse data structure of [`Component`](crate::component::Component)s.
///
/// Designed for relatively fast insertions and deletions.
pub struct ComponentSparseSet {
    /// This maps [`EntityRow`] to a [`TableRow`], an index in [`data`](Self::data).
    entity_to_row: Vec<Option<(SparseSetRow, EntityGeneration)>>,
    /// The rows that are free in [`data`](Self::data).
    ///
    /// # Safety
    ///
    /// These must be valid indecies into all the data and change detection lists.
    free_rows: Vec<SparseSetRow>,
    /// The length of the column buffer.
    buffer_len: usize,
    /// The capacity of the column buffer.
    buffer_capacity: usize,
    /// This is effectively a `Vec<MaybeUninit<MyComponent>>`, but it has the layout and drop for `MyComponent`.
    /// So, this needs it's drop to be temporarily disabled when dropping uninit data.
    column: ThinColumn,
}

impl ComponentSparseSet {
    /// Creates a new [`ComponentSparseSet`] with a given component type layout and
    /// initial `capacity`.
    pub(crate) fn new(component_info: &ComponentInfo, capacity: usize) -> Self {
        Self {
            entity_to_row: Vec::new(),
            free_rows: Vec::new(),
            column: ThinColumn::with_capacity(component_info, capacity),
            buffer_len: 0,
            buffer_capacity: 0,
        }
    }

    /// Removes all of the values stored within.
    pub(crate) fn clear(&mut self) {
        let Self {
            entity_to_row,
            free_rows,
            buffer_len,
            buffer_capacity: _,
            column,
        } = self;

        if let Some(drop) = column.data.drop {
            for row in entity_to_row.iter().filter_map(|row| row.map(|row| row.0)) {
                // SAFETY: We have &mut and clearing all rows. This value will never be accessed again.
                unsafe { drop(column.data.get_unchecked_mut(row.index()).promote()) }
            }
        }

        free_rows.clear();
        entity_to_row.clear();
        *buffer_len = 0;
    }

    /// Returns the number of component values in the sparse set.
    #[inline]
    pub fn len(&self) -> u32 {
        // There can't be more than u32::MAX entities, so the length is always less than that.
        (self.buffer_len - self.free_rows.len()) as u32
    }

    /// Returns `true` if the sparse set contains no component values.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.buffer_len == self.free_rows.len()
    }

    #[inline]
    fn get_row_of(&self, entity: Entity) -> Option<SparseSetRow> {
        self.entity_to_row
            .get(entity.index() as usize)
            .copied()
            .flatten()
            .and_then(|(row, generation)| (entity.generation() == generation).then_some(row))
    }

    #[inline]
    fn set_row_of(&mut self, entity: Entity, row: SparseSetRow) {
        self.entity_to_row.resize(
            self.entity_to_row.len().max((entity.index() + 1) as usize),
            None,
        );
        // SAFETY: We just resized
        unsafe {
            *self
                .entity_to_row
                .get_unchecked_mut(entity.index() as usize) = Some((row, entity.generation()));
        }
    }

    /// Reserves `additional` elements worth of capacity within the column buffer.
    #[inline]
    fn extend_buffer(&mut self, additional: usize) {
        self.buffer_len += additional;
        if self.buffer_len <= self.buffer_capacity {
            return;
        }

        if self.buffer_capacity == 0 {
            const STARTING_CAPACITY: usize = 256;
            // SAFETY: the current capacity is 0
            unsafe {
                self.column
                    .alloc(NonZeroUsize::new_unchecked(STARTING_CAPACITY));
            }
            self.buffer_capacity = STARTING_CAPACITY;
        } else {
            let mut new_capacity = self.buffer_capacity;
            while self.buffer_len > new_capacity {
                new_capacity *= 2;
            }
            self.buffer_capacity = new_capacity;

            // SAFETY:
            // - `column_cap` is indeed the columns' capacity
            unsafe {
                self.column.realloc(
                    NonZeroUsize::new_unchecked(self.buffer_capacity),
                    NonZeroUsize::new_unchecked(new_capacity),
                );
            };
        }
    }

    /// Inserts the `entity` key and component `value` pair into this sparse
    /// set.
    ///
    /// # Safety
    /// The `value` pointer must point to a valid address that matches the [`Layout`](std::alloc::Layout)
    /// inside the [`ComponentInfo`] given when constructing this sparse set.
    pub(crate) unsafe fn insert(
        &mut self,
        entity: Entity,
        value: OwningPtr<'_>,
        change_tick: Tick,
        caller: MaybeLocation,
    ) -> SparseSetRow {
        match self
            .entity_to_row
            .get(entity.index() as usize)
            .copied()
            .flatten()
        {
            Some((row, generation)) => {
                debug_assert_eq!(generation, entity.generation(), "An entity is being inserted into a sparse set, but the sparse set, still has an earlier generation of the entity!");
                // SAFETY: Caller ensures value is correct, row is in bounds,
                // the current value was initialized and present.
                unsafe {
                    self.column.replace(row.index(), value, change_tick, caller);
                }
                row
            }
            None => {
                if let Some(row) = self.free_rows.pop() {
                    self.set_row_of(entity, row);
                    // SAFETY: Caller ensures value is correct, free_rows ensures the row is in bound,
                    // and the previous value at the row was already dropped.
                    unsafe {
                        self.column
                            .initialize(row.index(), value, change_tick, caller);
                    }
                    row
                } else {
                    let idx = self.buffer_len as u32;
                    // SAFETY: There are never more than u32::MAX entity rows and this row was not present.
                    let row = unsafe { SparseSetRow::new(NonMaxU32::new_unchecked(idx)) };
                    self.set_row_of(entity, row);
                    self.extend_buffer(1);
                    // SAFETY: Caller ensures value is correct, and we just made the row in bounds.
                    unsafe {
                        self.column
                            .initialize(row.index(), value, change_tick, caller);
                    }
                    row
                }
            }
        }
    }

    /// Returns `true` if the sparse set has a component value for the provided `entity`.
    #[inline]
    pub fn contains(&self, entity: Entity) -> bool {
        self.get_row_of(entity).is_some()
    }

    /// Returns a reference to the entity's component value.
    ///
    /// Returns `None` if `entity` does not have a component in the sparse set.
    #[inline]
    pub fn get(&self, entity: Entity) -> Option<Ptr<'_>> {
        self.get_row_of(entity).map(|row| {
            // SAFETY: row is correct.
            unsafe { self.column.data.get_unchecked(row.index()) }
        })
    }

    /// Returns references to the entity's component value and its added and changed ticks.
    ///
    /// Returns `None` if `entity` does not have a component in the sparse set.
    #[inline]
    pub fn get_with_ticks(
        &self,
        entity: Entity,
    ) -> Option<(
        Ptr<'_>,
        TickCells<'_>,
        MaybeLocation<&UnsafeCell<&'static Location<'static>>>,
    )> {
        self.get_row_of(entity).map(|row| {
            // SAFETY: row is correct.
            unsafe {
                (
                    self.column.data.get_unchecked(row.index()),
                    TickCells {
                        added: self.column.added_ticks.get_unchecked(row.index()),
                        changed: self.column.changed_ticks.get_unchecked(row.index()),
                    },
                    self.column
                        .changed_by
                        .as_ref()
                        .map(|changed_by| changed_by.get_unchecked(row.index())),
                )
            }
        })
    }

    /// Returns a reference to the "added" tick of the entity's component value.
    ///
    /// Returns `None` if `entity` does not have a component in the sparse set.
    #[inline]
    pub fn get_added_tick(&self, entity: Entity) -> Option<&UnsafeCell<Tick>> {
        self.get_row_of(entity).map(|row| {
            // SAFETY: row is correct.
            unsafe { self.column.added_ticks.get_unchecked(row.index()) }
        })
    }

    /// Returns a reference to the "changed" tick of the entity's component value.
    ///
    /// Returns `None` if `entity` does not have a component in the sparse set.
    #[inline]
    pub fn get_changed_tick(&self, entity: Entity) -> Option<&UnsafeCell<Tick>> {
        self.get_row_of(entity).map(|row| {
            // SAFETY: row is correct.
            unsafe { self.column.changed_ticks.get_unchecked(row.index()) }
        })
    }

    /// Returns a reference to the "added" and "changed" ticks of the entity's component value.
    ///
    /// Returns `None` if `entity` does not have a component in the sparse set.
    #[inline]
    pub fn get_ticks(&self, entity: Entity) -> Option<ComponentTicks> {
        self.get_row_of(entity).map(|row| {
            // SAFETY: row is correct. We only change ticks with &mut self, and we have &self.
            unsafe {
                TickCells {
                    added: self.column.added_ticks.get_unchecked(row.index()),
                    changed: self.column.changed_ticks.get_unchecked(row.index()),
                }
                .read()
            }
        })
    }

    /// Returns a reference to the calling location that last changed the entity's component value.
    ///
    /// Returns `None` if `entity` does not have a component in the sparse set.
    #[inline]
    pub fn get_changed_by(
        &self,
        entity: Entity,
    ) -> MaybeLocation<Option<&UnsafeCell<&'static Location<'static>>>> {
        self.column.changed_by.as_ref().map(|changed_by| {
            self.get_row_of(entity).map(|row| {
                // SAFETY: row is correct.
                unsafe { changed_by.get_unchecked(row.index()) }
            })
        })
    }

    /// Returns the drop function for the component type stored in the sparse set,
    /// or `None` if it doesn't need to be dropped.
    #[inline]
    pub fn get_drop(&self) -> Option<unsafe fn(OwningPtr<'_>)> {
        self.column.data.drop
    }

    /// Removes the `entity` from this sparse set and returns a pointer to the associated value (if
    /// it exists).
    #[must_use = "The returned pointer must be used to drop the removed component."]
    pub(crate) fn remove_and_forget(&mut self, entity: Entity) -> Option<OwningPtr<'_>> {
        self.get_row_of(entity).map(|row| {
            self.free_rows.push(row);
            // SAFETY: The entity row must be in bounds or it wouldn't have had a sparse set row.
            unsafe {
                *self
                    .entity_to_row
                    .get_unchecked_mut(entity.index() as usize) = None;
            }
            // SAFETY: row is correct. We just freed this row, so nothing will ever access this value again.
            unsafe { self.column.data.get_unchecked_mut(row.index()).promote() }
        })
    }

    /// Removes (and drops) the entity's component value from the sparse set.
    ///
    /// Returns `true` if `entity` had a component value in the sparse set.
    pub(crate) fn remove(&mut self, entity: Entity) -> bool {
        let dropper = self.get_drop();
        match self.remove_and_forget(entity) {
            Some(to_drop) => {
                if let Some(dropper) = dropper {
                    // SAFETY: We have authority to drop this.
                    unsafe { dropper(to_drop) }
                }
                true
            }
            None => false,
        }
    }

    pub(crate) fn check_change_ticks(&mut self, change_tick: Tick) {
        let Self {
            entity_to_row,
            free_rows: _,
            buffer_len: _,
            buffer_capacity: _,
            column,
        } = self;
        for row in entity_to_row.iter().filter_map(|row| row.map(|row| row.0)) {
            // SAFETY: We have &mut and the row is in bounds
            unsafe {
                column
                    .added_ticks
                    .get_unchecked_mut(row.index())
                    .get_mut()
                    .check_tick(change_tick);

                column
                    .changed_ticks
                    .get_unchecked_mut(row.index())
                    .get_mut()
                    .check_tick(change_tick);
            }
        }
    }
}

impl Drop for ComponentSparseSet {
    fn drop(&mut self) {
        let Self {
            entity_to_row,
            free_rows: _,
            buffer_len,
            buffer_capacity,
            column,
        } = self;

        if let Some(drop) = column.data.drop {
            for row in entity_to_row.iter().filter_map(|row| row.map(|row| row.0)) {
                // SAFETY: We have &mut and are being dropped
                unsafe { drop(column.data.get_unchecked_mut(row.index()).promote()) }
            }
        }

        // SAFETY: Values are correct, and it is being dropped.
        unsafe {
            column.drop_and_forget_data(*buffer_capacity, *buffer_len);
        }
    }
}

/// A data structure that blends dense and sparse storage
///
/// `I` is the type of the indices, while `V` is the type of data stored in the dense storage.
#[derive(Debug)]
pub struct SparseSet<I, V: 'static> {
    dense: Vec<V>,
    indices: Vec<I>,
    sparse: SparseArray<I, NonMaxUsize>,
}

/// A space-optimized version of [`SparseSet`] that cannot be changed
/// after construction.
#[derive(Debug)]
pub(crate) struct ImmutableSparseSet<I, V: 'static> {
    dense: Box<[V]>,
    indices: Box<[I]>,
    sparse: ImmutableSparseArray<I, NonMaxUsize>,
}

macro_rules! impl_sparse_set {
    ($ty:ident) => {
        impl<I: SparseSetIndex, V> $ty<I, V> {
            /// Returns the number of elements in the sparse set.
            #[inline]
            pub fn len(&self) -> usize {
                self.dense.len()
            }

            /// Returns `true` if the sparse set contains a value for `index`.
            #[inline]
            pub fn contains(&self, index: I) -> bool {
                self.sparse.contains(index)
            }

            /// Returns a reference to the value for `index`.
            ///
            /// Returns `None` if `index` does not have a value in the sparse set.
            pub fn get(&self, index: I) -> Option<&V> {
                self.sparse.get(index).map(|dense_index| {
                    // SAFETY: if the sparse index points to something in the dense vec, it exists
                    unsafe { self.dense.get_unchecked(dense_index.get()) }
                })
            }

            /// Returns a mutable reference to the value for `index`.
            ///
            /// Returns `None` if `index` does not have a value in the sparse set.
            pub fn get_mut(&mut self, index: I) -> Option<&mut V> {
                let dense = &mut self.dense;
                self.sparse.get(index).map(move |dense_index| {
                    // SAFETY: if the sparse index points to something in the dense vec, it exists
                    unsafe { dense.get_unchecked_mut(dense_index.get()) }
                })
            }

            /// Returns an iterator visiting all keys (indices) in arbitrary order.
            pub fn indices(&self) -> impl Iterator<Item = I> + Clone + '_ {
                self.indices.iter().cloned()
            }

            /// Returns an iterator visiting all values in arbitrary order.
            pub fn values(&self) -> impl Iterator<Item = &V> {
                self.dense.iter()
            }

            /// Returns an iterator visiting all values mutably in arbitrary order.
            pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
                self.dense.iter_mut()
            }

            /// Returns an iterator visiting all key-value pairs in arbitrary order, with references to the values.
            pub fn iter(&self) -> impl Iterator<Item = (&I, &V)> {
                self.indices.iter().zip(self.dense.iter())
            }

            /// Returns an iterator visiting all key-value pairs in arbitrary order, with mutable references to the values.
            pub fn iter_mut(&mut self) -> impl Iterator<Item = (&I, &mut V)> {
                self.indices.iter().zip(self.dense.iter_mut())
            }
        }
    };
}

impl_sparse_set!(SparseSet);
impl_sparse_set!(ImmutableSparseSet);

impl<I: SparseSetIndex, V> Default for SparseSet<I, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<I, V> SparseSet<I, V> {
    /// Creates a new [`SparseSet`].
    pub const fn new() -> Self {
        Self {
            dense: Vec::new(),
            indices: Vec::new(),
            sparse: SparseArray::new(),
        }
    }
}

impl<I: SparseSetIndex, V> SparseSet<I, V> {
    /// Creates a new [`SparseSet`] with a specified initial capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            dense: Vec::with_capacity(capacity),
            indices: Vec::with_capacity(capacity),
            sparse: Default::default(),
        }
    }

    /// Returns the total number of elements the [`SparseSet`] can hold without needing to reallocate.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.dense.capacity()
    }

    /// Inserts `value` at `index`.
    ///
    /// If a value was already present at `index`, it will be overwritten.
    pub fn insert(&mut self, index: I, value: V) {
        if let Some(dense_index) = self.sparse.get(index.clone()).cloned() {
            // SAFETY: dense indices stored in self.sparse always exist
            unsafe {
                *self.dense.get_unchecked_mut(dense_index.get()) = value;
            }
        } else {
            self.sparse
                .insert(index.clone(), NonMaxUsize::new(self.dense.len()).unwrap());
            self.indices.push(index);
            self.dense.push(value);
        }
    }

    /// Returns a reference to the value for `index`, inserting one computed from `func`
    /// if not already present.
    pub fn get_or_insert_with(&mut self, index: I, func: impl FnOnce() -> V) -> &mut V {
        if let Some(dense_index) = self.sparse.get(index.clone()).cloned() {
            // SAFETY: dense indices stored in self.sparse always exist
            unsafe { self.dense.get_unchecked_mut(dense_index.get()) }
        } else {
            let value = func();
            let dense_index = self.dense.len();
            self.sparse
                .insert(index.clone(), NonMaxUsize::new(dense_index).unwrap());
            self.indices.push(index);
            self.dense.push(value);
            // SAFETY: dense index was just populated above
            unsafe { self.dense.get_unchecked_mut(dense_index) }
        }
    }

    /// Returns `true` if the sparse set contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.dense.len() == 0
    }

    /// Removes and returns the value for `index`.
    ///
    /// Returns `None` if `index` does not have a value in the sparse set.
    pub fn remove(&mut self, index: I) -> Option<V> {
        self.sparse.remove(index).map(|dense_index| {
            let index = dense_index.get();
            let is_last = index == self.dense.len() - 1;
            let value = self.dense.swap_remove(index);
            self.indices.swap_remove(index);
            if !is_last {
                let swapped_index = self.indices[index].clone();
                *self.sparse.get_mut(swapped_index).unwrap() = dense_index;
            }
            value
        })
    }

    /// Clears all of the elements from the sparse set.
    pub fn clear(&mut self) {
        self.dense.clear();
        self.indices.clear();
        self.sparse.clear();
    }

    /// Converts the sparse set into its immutable variant.
    pub(crate) fn into_immutable(self) -> ImmutableSparseSet<I, V> {
        ImmutableSparseSet {
            dense: self.dense.into_boxed_slice(),
            indices: self.indices.into_boxed_slice(),
            sparse: self.sparse.into_immutable(),
        }
    }
}

/// Represents something that can be stored in a [`SparseSet`] as an integer.
///
/// Ideally, the `usize` values should be very small (ie: incremented starting from
/// zero), as the number of bits needed to represent a `SparseSetIndex` in a `FixedBitSet`
/// is proportional to the **value** of those `usize`.
pub trait SparseSetIndex: Clone + PartialEq + Eq + Hash {
    /// Gets the sparse set index corresponding to this instance.
    fn sparse_set_index(&self) -> usize;
    /// Creates a new instance of this type with the specified index.
    fn get_sparse_set_index(value: usize) -> Self;
}

macro_rules! impl_sparse_set_index {
    ($($ty:ty),+) => {
        $(impl SparseSetIndex for $ty {
            #[inline]
            fn sparse_set_index(&self) -> usize {
                *self as usize
            }

            #[inline]
            fn get_sparse_set_index(value: usize) -> Self {
                value as $ty
            }
        })*
    };
}

impl_sparse_set_index!(u8, u16, u32, u64, usize);

/// A collection of [`ComponentSparseSet`] storages, indexed by [`ComponentId`]
///
/// Can be accessed via [`Storages`](crate::storage::Storages)
#[derive(Default)]
pub struct SparseSets {
    sets: SparseSet<ComponentId, ComponentSparseSet>,
}

impl SparseSets {
    /// Returns the number of [`ComponentSparseSet`]s this collection contains.
    #[inline]
    pub fn len(&self) -> usize {
        self.sets.len()
    }

    /// Returns true if this collection contains no [`ComponentSparseSet`]s.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.sets.is_empty()
    }

    /// An Iterator visiting all ([`ComponentId`], [`ComponentSparseSet`]) pairs.
    /// NOTE: Order is not guaranteed.
    pub fn iter(&self) -> impl Iterator<Item = (ComponentId, &ComponentSparseSet)> {
        self.sets.iter().map(|(id, data)| (*id, data))
    }

    /// Gets a reference to the [`ComponentSparseSet`] of a [`ComponentId`]. This may be `None` if the component has never been spawned.
    #[inline]
    pub fn get(&self, component_id: ComponentId) -> Option<&ComponentSparseSet> {
        self.sets.get(component_id)
    }

    /// Gets a mutable reference of [`ComponentSparseSet`] of a [`ComponentInfo`].
    /// Create a new [`ComponentSparseSet`] if not exists.
    pub(crate) fn get_or_insert(
        &mut self,
        component_info: &ComponentInfo,
    ) -> &mut ComponentSparseSet {
        if !self.sets.contains(component_info.id()) {
            self.sets.insert(
                component_info.id(),
                ComponentSparseSet::new(component_info, 64),
            );
        }

        self.sets.get_mut(component_info.id()).unwrap()
    }

    /// Gets a mutable reference to the [`ComponentSparseSet`] of a [`ComponentId`]. This may be `None` if the component has never been spawned.
    pub(crate) fn get_mut(&mut self, component_id: ComponentId) -> Option<&mut ComponentSparseSet> {
        self.sets.get_mut(component_id)
    }

    /// Clear entities stored in each [`ComponentSparseSet`]
    pub(crate) fn clear_entities(&mut self) {
        for set in self.sets.values_mut() {
            set.clear();
        }
    }

    pub(crate) fn check_change_ticks(&mut self, change_tick: Tick) {
        for set in self.sets.values_mut() {
            set.check_change_ticks(change_tick);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SparseSets;
    use crate::{
        component::{Component, ComponentDescriptor, ComponentId, ComponentInfo},
        entity::{Entity, EntityRow},
        storage::SparseSet,
    };
    use alloc::{vec, vec::Vec};
    use nonmax::NonMaxU32;

    #[derive(Debug, Eq, PartialEq)]
    struct Foo(usize);

    #[test]
    fn sparse_set() {
        let mut set = SparseSet::<Entity, Foo>::default();
        let e0 = Entity::from_raw(EntityRow::new(NonMaxU32::new(0).unwrap()));
        let e1 = Entity::from_raw(EntityRow::new(NonMaxU32::new(1).unwrap()));
        let e2 = Entity::from_raw(EntityRow::new(NonMaxU32::new(2).unwrap()));
        let e3 = Entity::from_raw(EntityRow::new(NonMaxU32::new(3).unwrap()));
        let e4 = Entity::from_raw(EntityRow::new(NonMaxU32::new(4).unwrap()));

        set.insert(e1, Foo(1));
        set.insert(e2, Foo(2));
        set.insert(e3, Foo(3));

        assert_eq!(set.get(e0), None);
        assert_eq!(set.get(e1), Some(&Foo(1)));
        assert_eq!(set.get(e2), Some(&Foo(2)));
        assert_eq!(set.get(e3), Some(&Foo(3)));
        assert_eq!(set.get(e4), None);

        {
            let iter_results = set.values().collect::<Vec<_>>();
            assert_eq!(iter_results, vec![&Foo(1), &Foo(2), &Foo(3)]);
        }

        assert_eq!(set.remove(e2), Some(Foo(2)));
        assert_eq!(set.remove(e2), None);

        assert_eq!(set.get(e0), None);
        assert_eq!(set.get(e1), Some(&Foo(1)));
        assert_eq!(set.get(e2), None);
        assert_eq!(set.get(e3), Some(&Foo(3)));
        assert_eq!(set.get(e4), None);

        assert_eq!(set.remove(e1), Some(Foo(1)));

        assert_eq!(set.get(e0), None);
        assert_eq!(set.get(e1), None);
        assert_eq!(set.get(e2), None);
        assert_eq!(set.get(e3), Some(&Foo(3)));
        assert_eq!(set.get(e4), None);

        set.insert(e1, Foo(10));

        assert_eq!(set.get(e1), Some(&Foo(10)));

        *set.get_mut(e1).unwrap() = Foo(11);
        assert_eq!(set.get(e1), Some(&Foo(11)));
    }

    #[test]
    fn sparse_sets() {
        let mut sets = SparseSets::default();

        #[derive(Component, Default, Debug)]
        struct TestComponent1;

        #[derive(Component, Default, Debug)]
        struct TestComponent2;

        assert_eq!(sets.len(), 0);
        assert!(sets.is_empty());

        register_component::<TestComponent1>(&mut sets, 1);
        assert_eq!(sets.len(), 1);

        register_component::<TestComponent2>(&mut sets, 2);
        assert_eq!(sets.len(), 2);

        // check its shape by iter
        let mut collected_sets = sets
            .iter()
            .map(|(id, set)| (id, set.len()))
            .collect::<Vec<_>>();
        collected_sets.sort();
        assert_eq!(
            collected_sets,
            vec![(ComponentId::new(1), 0), (ComponentId::new(2), 0),]
        );

        fn register_component<T: Component>(sets: &mut SparseSets, id: usize) {
            let descriptor = ComponentDescriptor::new::<T>();
            let id = ComponentId::new(id);
            let info = ComponentInfo::new(id, descriptor);
            sets.get_or_insert(&info);
        }
    }
}

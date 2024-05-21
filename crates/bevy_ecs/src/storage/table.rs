use crate::{
    component::{ComponentId, ComponentInfo, ComponentTicks, Components, Tick, TickCells},
    entity::Entity,
    query::DebugCheckedUnwrap,
    storage::{blob_vec::BlobVec, ImmutableSparseSet, SparseSet},
};
use bevy_ptr::{OwningPtr, Ptr, PtrMut, UnsafeCellDeref};
use bevy_utils::HashMap;
use std::alloc::Layout;
use std::{
    cell::UnsafeCell,
    ops::{Index, IndexMut},
};

/// An opaque unique ID for a [`Table`] within a [`World`].
///
/// Can be used with [`Tables::get`] to fetch the corresponding
/// table.
///
/// Each [`Archetype`] always points to a table via [`Archetype::table_id`].
/// Multiple archetypes can point to the same table so long as the components
/// stored in the table are identical, but do not share the same sparse set
/// components.
///
/// [`World`]: crate::world::World
/// [`Archetype`]: crate::archetype::Archetype
/// [`Archetype::table_id`]: crate::archetype::Archetype::table_id
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// SAFETY: Must be repr(transparent) due to the safety requirements on EntityLocation
#[repr(transparent)]
pub struct TableId(u32);

impl TableId {
    pub(crate) const INVALID: TableId = TableId(u32::MAX);

    /// Creates a new [`TableId`].
    ///
    /// `index` *must* be retrieved from calling [`TableId::as_u32`] on a `TableId` you got
    /// from a table of a given [`World`] or the created ID may be invalid.
    ///
    /// [`World`]: crate::world::World
    #[inline]
    pub const fn from_u32(index: u32) -> Self {
        Self(index)
    }

    /// Creates a new [`TableId`].
    ///
    /// `index` *must* be retrieved from calling [`TableId::as_usize`] on a `TableId` you got
    /// from a table of a given [`World`] or the created ID may be invalid.
    ///
    /// [`World`]: crate::world::World
    ///
    /// # Panics
    ///
    /// Will panic if the provided value does not fit within a [`u32`].
    #[inline]
    pub const fn from_usize(index: usize) -> Self {
        debug_assert!(index as u32 as usize == index);
        Self(index as u32)
    }

    /// Gets the underlying table index from the ID.
    #[inline]
    pub const fn as_u32(self) -> u32 {
        self.0
    }

    /// Gets the underlying table index from the ID.
    #[inline]
    pub const fn as_usize(self) -> usize {
        // usize is at least u32 in Bevy
        self.0 as usize
    }

    /// The [`TableId`] of the [`Table`] without any components.
    #[inline]
    pub const fn empty() -> Self {
        Self(0)
    }
}

/// A opaque newtype for rows in [`Table`]s. Specifies a single row in a specific table.
///
/// Values of this type are retrievable from [`Archetype::entity_table_row`] and can be
/// used alongside [`Archetype::table_id`] to fetch the exact table and row where an
/// [`Entity`]'s
///
/// Values of this type are only valid so long as entities have not moved around.
/// Adding and removing components from an entity, or despawning it will invalidate
/// potentially any table row in the table the entity was previously stored in. Users
/// should *always* fetch the appropriate row from the entity's [`Archetype`] before
/// fetching the entity's components.
///
/// [`Archetype`]: crate::archetype::Archetype
/// [`Archetype::entity_table_row`]: crate::archetype::Archetype::entity_table_row
/// [`Archetype::table_id`]: crate::archetype::Archetype::table_id
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// SAFETY: Must be repr(transparent) due to the safety requirements on EntityLocation
#[repr(transparent)]
pub struct TableRow(u32);

impl TableRow {
    pub(crate) const INVALID: TableRow = TableRow(u32::MAX);

    /// Creates a `TableRow`.
    #[inline]
    pub const fn from_u32(index: u32) -> Self {
        Self(index)
    }

    /// Creates a `TableRow` from a [`usize`] index.
    ///
    /// # Panics
    ///
    /// Will panic if the provided value does not fit within a [`u32`].
    #[inline]
    pub const fn from_usize(index: usize) -> Self {
        debug_assert!(index as u32 as usize == index);
        Self(index as u32)
    }

    /// Gets the index of the row as a [`usize`].
    #[inline]
    pub const fn as_usize(self) -> usize {
        // usize is at least u32 in Bevy
        self.0 as usize
    }

    /// Gets the index of the row as a [`usize`].
    #[inline]
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

/// A type-erased contiguous container for data of a homogeneous type.
///
/// Conceptually, a [`Column`] is very similar to a type-erased `Vec<T>`.
/// It also stores the change detection ticks for its components, kept in two separate
/// contiguous buffers internally. An element shares its data across these buffers by using the
/// same index (i.e. the entity at row 3 has its data at index 3 and its change detection ticks at
/// index 3). A slice to these contiguous blocks of memory can be fetched
/// via [`Column::get_data_slice`], [`Column::get_added_ticks_slice`], and
/// [`Column::get_changed_ticks_slice`].
///
/// Like many other low-level storage types, [`Column`] has a limited and highly unsafe
/// interface. It's highly advised to use higher level types and their safe abstractions
/// instead of working directly with [`Column`].
#[derive(Debug)]
pub struct Column {
    data: BlobVec,
    added_ticks: Vec<UnsafeCell<Tick>>,
    changed_ticks: Vec<UnsafeCell<Tick>>,
}

impl Column {
    /// Constructs a new [`Column`], configured with a component's layout and an initial `capacity`.
    #[inline]
    pub(crate) fn with_capacity(component_info: &ComponentInfo, capacity: usize) -> Self {
        Column {
            // SAFETY: component_info.drop() is valid for the types that will be inserted.
            data: unsafe { BlobVec::new(component_info.layout(), component_info.drop(), capacity) },
            added_ticks: Vec::with_capacity(capacity),
            changed_ticks: Vec::with_capacity(capacity),
        }
    }

    /// Fetches the [`Layout`] for the underlying type.
    #[inline]
    pub fn item_layout(&self) -> Layout {
        self.data.layout()
    }

    /// Writes component data to the column at given row.
    /// Assumes the slot is uninitialized, drop is not called.
    /// To overwrite existing initialized value, use `replace` instead.
    ///
    /// # Safety
    /// Assumes data has already been allocated for the given row.
    #[inline]
    pub(crate) unsafe fn initialize(&mut self, row: TableRow, data: OwningPtr<'_>, tick: Tick) {
        debug_assert!(row.as_usize() < self.len());
        self.data.initialize_unchecked(row.as_usize(), data);
        *self.added_ticks.get_unchecked_mut(row.as_usize()).get_mut() = tick;
        *self
            .changed_ticks
            .get_unchecked_mut(row.as_usize())
            .get_mut() = tick;
    }

    /// Writes component data to the column at given row.
    /// Assumes the slot is initialized, calls drop.
    ///
    /// # Safety
    /// Assumes data has already been allocated for the given row.
    #[inline]
    pub(crate) unsafe fn replace(&mut self, row: TableRow, data: OwningPtr<'_>, change_tick: Tick) {
        debug_assert!(row.as_usize() < self.len());
        self.data.replace_unchecked(row.as_usize(), data);
        *self
            .changed_ticks
            .get_unchecked_mut(row.as_usize())
            .get_mut() = change_tick;
    }

    /// Gets the current number of elements stored in the column.
    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Checks if the column is empty. Returns `true` if there are no elements, `false` otherwise.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Removes an element from the [`Column`].
    ///
    /// - The value will be dropped if it implements [`Drop`].
    /// - This does not preserve ordering, but is O(1).
    /// - This does not do any bounds checking.
    /// - The element is replaced with the last element in the [`Column`].
    ///
    /// # Safety
    /// `row` must be within the range `[0, self.len())`.
    ///
    #[inline]
    pub(crate) unsafe fn swap_remove_unchecked(&mut self, row: TableRow) {
        self.data.swap_remove_and_drop_unchecked(row.as_usize());
        self.added_ticks.swap_remove(row.as_usize());
        self.changed_ticks.swap_remove(row.as_usize());
    }

    /// Removes an element from the [`Column`] and returns it and its change detection ticks.
    /// This does not preserve ordering, but is O(1) and does not do any bounds checking.
    ///
    /// The element is replaced with the last element in the [`Column`].
    ///
    /// It's the caller's responsibility to ensure that the removed value is dropped or used.
    /// Failure to do so may result in resources not being released (i.e. files handles not being
    /// released, memory leaks, etc.)
    ///
    /// # Safety
    /// `row` must be within the range `[0, self.len())`.
    #[inline]
    #[must_use = "The returned pointer should be used to dropped the removed component"]
    pub(crate) unsafe fn swap_remove_and_forget_unchecked(
        &mut self,
        row: TableRow,
    ) -> (OwningPtr<'_>, ComponentTicks) {
        let data = self.data.swap_remove_and_forget_unchecked(row.as_usize());
        let added = self.added_ticks.swap_remove(row.as_usize()).into_inner();
        let changed = self.changed_ticks.swap_remove(row.as_usize()).into_inner();
        (data, ComponentTicks { added, changed })
    }

    /// Removes the element from `other` at `src_row` and inserts it
    /// into the current column to initialize the values at `dst_row`.
    /// Does not do any bounds checking.
    ///
    /// # Safety
    ///
    ///  - `other` must have the same data layout as `self`
    ///  - `src_row` must be in bounds for `other`
    ///  - `dst_row` must be in bounds for `self`
    ///  - `other[src_row]` must be initialized to a valid value.
    ///  - `self[dst_row]` must not be initialized yet.
    #[inline]
    pub(crate) unsafe fn initialize_from_unchecked(
        &mut self,
        other: &mut Column,
        src_row: TableRow,
        dst_row: TableRow,
    ) {
        debug_assert!(self.data.layout() == other.data.layout());
        let ptr = self.data.get_unchecked_mut(dst_row.as_usize());
        other.data.swap_remove_unchecked(src_row.as_usize(), ptr);
        *self.added_ticks.get_unchecked_mut(dst_row.as_usize()) =
            other.added_ticks.swap_remove(src_row.as_usize());
        *self.changed_ticks.get_unchecked_mut(dst_row.as_usize()) =
            other.changed_ticks.swap_remove(src_row.as_usize());
    }

    /// Pushes a new value onto the end of the [`Column`].
    ///
    /// # Safety
    /// `ptr` must point to valid data of this column's component type
    pub(crate) unsafe fn push(&mut self, ptr: OwningPtr<'_>, ticks: ComponentTicks) {
        self.data.push(ptr);
        self.added_ticks.push(UnsafeCell::new(ticks.added));
        self.changed_ticks.push(UnsafeCell::new(ticks.changed));
    }

    #[inline]
    pub(crate) fn reserve_exact(&mut self, additional: usize) {
        self.data.reserve_exact(additional);
        self.added_ticks.reserve_exact(additional);
        self.changed_ticks.reserve_exact(additional);
    }

    /// Fetches the data pointer to the first element of the [`Column`].
    ///
    /// The pointer is type erased, so using this function to fetch anything
    /// other than the first element will require computing the offset using
    /// [`Column::item_layout`].
    #[inline]
    pub fn get_data_ptr(&self) -> Ptr<'_> {
        self.data.get_ptr()
    }

    /// Fetches the slice to the [`Column`]'s data cast to a given type.
    ///
    /// Note: The values stored within are [`UnsafeCell`].
    /// Users of this API must ensure that accesses to each individual element
    /// adhere to the safety invariants of [`UnsafeCell`].
    ///
    /// # Safety
    /// The type `T` must be the type of the items in this column.
    pub unsafe fn get_data_slice<T>(&self) -> &[UnsafeCell<T>] {
        self.data.get_slice()
    }

    /// Fetches the slice to the [`Column`]'s "added" change detection ticks.
    ///
    /// Note: The values stored within are [`UnsafeCell`].
    /// Users of this API must ensure that accesses to each individual element
    /// adhere to the safety invariants of [`UnsafeCell`].
    #[inline]
    pub fn get_added_ticks_slice(&self) -> &[UnsafeCell<Tick>] {
        &self.added_ticks
    }

    /// Fetches the slice to the [`Column`]'s "changed" change detection ticks.
    ///
    /// Note: The values stored within are [`UnsafeCell`].
    /// Users of this API must ensure that accesses to each individual element
    /// adhere to the safety invariants of [`UnsafeCell`].
    #[inline]
    pub fn get_changed_ticks_slice(&self) -> &[UnsafeCell<Tick>] {
        &self.changed_ticks
    }

    /// Fetches a reference to the data and change detection ticks at `row`.
    ///
    /// Returns `None` if `row` is out of bounds.
    #[inline]
    pub fn get(&self, row: TableRow) -> Option<(Ptr<'_>, TickCells<'_>)> {
        (row.as_usize() < self.data.len())
            // SAFETY: The row is length checked before fetching the pointer. This is being
            // accessed through a read-only reference to the column.
            .then(|| unsafe {
                (
                    self.data.get_unchecked(row.as_usize()),
                    TickCells {
                        added: self.added_ticks.get_unchecked(row.as_usize()),
                        changed: self.changed_ticks.get_unchecked(row.as_usize()),
                    },
                )
            })
    }

    /// Fetches a read-only reference to the data at `row`.
    ///
    /// Returns `None` if `row` is out of bounds.
    #[inline]
    pub fn get_data(&self, row: TableRow) -> Option<Ptr<'_>> {
        (row.as_usize() < self.data.len()).then(|| {
            // SAFETY: The row is length checked before fetching the pointer. This is being
            // accessed through a read-only reference to the column.
            unsafe { self.data.get_unchecked(row.as_usize()) }
        })
    }

    /// Fetches a read-only reference to the data at `row`. Unlike [`Column::get`] this does not
    /// do any bounds checking.
    ///
    /// # Safety
    /// - `row` must be within the range `[0, self.len())`.
    /// - no other mutable reference to the data of the same row can exist at the same time
    #[inline]
    pub unsafe fn get_data_unchecked(&self, row: TableRow) -> Ptr<'_> {
        debug_assert!(row.as_usize() < self.data.len());
        self.data.get_unchecked(row.as_usize())
    }

    /// Fetches a mutable reference to the data at `row`.
    ///
    /// Returns `None` if `row` is out of bounds.
    #[inline]
    pub fn get_data_mut(&mut self, row: TableRow) -> Option<PtrMut<'_>> {
        (row.as_usize() < self.data.len()).then(|| {
            // SAFETY: The row is length checked before fetching the pointer. This is being
            // accessed through an exclusive reference to the column.
            unsafe { self.data.get_unchecked_mut(row.as_usize()) }
        })
    }

    /// Fetches a mutable reference to the data at `row`. Unlike [`Column::get_data_mut`] this does not
    /// do any bounds checking.
    ///
    /// # Safety
    /// - index must be in-bounds
    /// - no other reference to the data of the same row can exist at the same time
    #[inline]
    pub(crate) unsafe fn get_data_unchecked_mut(&mut self, row: TableRow) -> PtrMut<'_> {
        debug_assert!(row.as_usize() < self.data.len());
        self.data.get_unchecked_mut(row.as_usize())
    }

    /// Fetches the "added" change detection tick for the value at `row`.
    ///
    /// Returns `None` if `row` is out of bounds.
    ///
    /// Note: The values stored within are [`UnsafeCell`].
    /// Users of this API must ensure that accesses to each individual element
    /// adhere to the safety invariants of [`UnsafeCell`].
    #[inline]
    pub fn get_added_tick(&self, row: TableRow) -> Option<&UnsafeCell<Tick>> {
        self.added_ticks.get(row.as_usize())
    }

    /// Fetches the "changed" change detection tick for the value at `row`.
    ///
    /// Returns `None` if `row` is out of bounds.
    ///
    /// Note: The values stored within are [`UnsafeCell`].
    /// Users of this API must ensure that accesses to each individual element
    /// adhere to the safety invariants of [`UnsafeCell`].
    #[inline]
    pub fn get_changed_tick(&self, row: TableRow) -> Option<&UnsafeCell<Tick>> {
        self.changed_ticks.get(row.as_usize())
    }

    /// Fetches the change detection ticks for the value at `row`.
    ///
    /// Returns `None` if `row` is out of bounds.
    #[inline]
    pub fn get_ticks(&self, row: TableRow) -> Option<ComponentTicks> {
        if row.as_usize() < self.data.len() {
            // SAFETY: The size of the column has already been checked.
            Some(unsafe { self.get_ticks_unchecked(row) })
        } else {
            None
        }
    }

    /// Fetches the "added" change detection tick for the value at `row`. Unlike [`Column::get_added_tick`]
    /// this function does not do any bounds checking.
    ///
    /// # Safety
    /// `row` must be within the range `[0, self.len())`.
    #[inline]
    pub unsafe fn get_added_tick_unchecked(&self, row: TableRow) -> &UnsafeCell<Tick> {
        debug_assert!(row.as_usize() < self.added_ticks.len());
        self.added_ticks.get_unchecked(row.as_usize())
    }

    /// Fetches the "changed" change detection tick for the value at `row`. Unlike [`Column::get_changed_tick`]
    /// this function does not do any bounds checking.
    ///
    /// # Safety
    /// `row` must be within the range `[0, self.len())`.
    #[inline]
    pub unsafe fn get_changed_tick_unchecked(&self, row: TableRow) -> &UnsafeCell<Tick> {
        debug_assert!(row.as_usize() < self.changed_ticks.len());
        self.changed_ticks.get_unchecked(row.as_usize())
    }

    /// Fetches the change detection ticks for the value at `row`. Unlike [`Column::get_ticks`]
    /// this function does not do any bounds checking.
    ///
    /// # Safety
    /// `row` must be within the range `[0, self.len())`.
    #[inline]
    pub unsafe fn get_ticks_unchecked(&self, row: TableRow) -> ComponentTicks {
        debug_assert!(row.as_usize() < self.added_ticks.len());
        debug_assert!(row.as_usize() < self.changed_ticks.len());
        ComponentTicks {
            added: self.added_ticks.get_unchecked(row.as_usize()).read(),
            changed: self.changed_ticks.get_unchecked(row.as_usize()).read(),
        }
    }

    /// Clears the column, removing all values.
    ///
    /// Note that this function has no effect on the allocated capacity of the [`Column`]>
    pub fn clear(&mut self) {
        self.data.clear();
        self.added_ticks.clear();
        self.changed_ticks.clear();
    }

    #[inline]
    pub(crate) fn check_change_ticks(&mut self, change_tick: Tick) {
        for component_ticks in &mut self.added_ticks {
            component_ticks.get_mut().check_tick(change_tick);
        }
        for component_ticks in &mut self.changed_ticks {
            component_ticks.get_mut().check_tick(change_tick);
        }
    }
}

/// A builder type for constructing [`Table`]s.
///
///  - Use [`with_capacity`] to initialize the builder.
///  - Repeatedly call [`add_column`] to add columns for components.
///  - Finalize with [`build`] to get the constructed [`Table`].
///
/// [`with_capacity`]: Self::with_capacity
/// [`add_column`]: Self::add_column
/// [`build`]: Self::build
pub(crate) struct TableBuilder {
    columns: SparseSet<ComponentId, Column>,
    capacity: usize,
}

impl TableBuilder {
    /// Creates a blank [`Table`], allocating space for `column_capacity` columns
    /// with the capacity to hold `capacity` entities worth of components each.
    pub fn with_capacity(capacity: usize, column_capacity: usize) -> Self {
        Self {
            columns: SparseSet::with_capacity(column_capacity),
            capacity,
        }
    }

    #[must_use]
    pub fn add_column(mut self, component_info: &ComponentInfo) -> Self {
        self.columns.insert(
            component_info.id(),
            Column::with_capacity(component_info, self.capacity),
        );
        self
    }

    #[must_use]
    pub fn build(self) -> Table {
        Table {
            columns: self.columns.into_immutable(),
            entities: Vec::with_capacity(self.capacity),
        }
    }
}

/// A column-oriented [structure-of-arrays] based storage for [`Component`]s of entities
/// in a [`World`].
///
/// Conceptually, a `Table` can be thought of as an `HashMap<ComponentId, Column>`, where
/// each [`Column`] is a type-erased `Vec<T: Component>`. Each row corresponds to a single entity
/// (i.e. index 3 in Column A and index 3 in Column B point to different components on the same
/// entity). Fetching components from a table involves fetching the associated column for a
/// component type (via its [`ComponentId`]), then fetching the entity's row within that column.
///
/// [structure-of-arrays]: https://en.wikipedia.org/wiki/AoS_and_SoA#Structure_of_arrays
/// [`Component`]: crate::component::Component
/// [`World`]: crate::world::World
pub struct Table {
    columns: ImmutableSparseSet<ComponentId, Column>,
    entities: Vec<Entity>,
}

impl Table {
    /// Fetches a read-only slice of the entities stored within the [`Table`].
    #[inline]
    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }

    /// Removes the entity at the given row and returns the entity swapped in to replace it (if an
    /// entity was swapped in)
    ///
    /// # Safety
    /// `row` must be in-bounds
    pub(crate) unsafe fn swap_remove_unchecked(&mut self, row: TableRow) -> Option<Entity> {
        for column in self.columns.values_mut() {
            column.swap_remove_unchecked(row);
        }
        let is_last = row.as_usize() == self.entities.len() - 1;
        self.entities.swap_remove(row.as_usize());
        if is_last {
            None
        } else {
            Some(self.entities[row.as_usize()])
        }
    }

    /// Moves the `row` column values to `new_table`, for the columns shared between both tables.
    /// Returns the index of the new row in `new_table` and the entity in this table swapped in
    /// to replace it (if an entity was swapped in). missing columns will be "forgotten". It is
    /// the caller's responsibility to drop them.  Failure to do so may result in resources not
    /// being released (i.e. files handles not being released, memory leaks, etc.)
    ///
    /// # Safety
    /// Row must be in-bounds
    pub(crate) unsafe fn move_to_and_forget_missing_unchecked(
        &mut self,
        row: TableRow,
        new_table: &mut Table,
    ) -> TableMoveResult {
        debug_assert!(row.as_usize() < self.entity_count());
        let is_last = row.as_usize() == self.entities.len() - 1;
        let new_row = new_table.allocate(self.entities.swap_remove(row.as_usize()));
        for (component_id, column) in self.columns.iter_mut() {
            if let Some(new_column) = new_table.get_column_mut(*component_id) {
                new_column.initialize_from_unchecked(column, row, new_row);
            } else {
                // It's the caller's responsibility to drop these cases.
                let (_, _) = column.swap_remove_and_forget_unchecked(row);
            }
        }
        TableMoveResult {
            new_row,
            swapped_entity: if is_last {
                None
            } else {
                Some(self.entities[row.as_usize()])
            },
        }
    }

    /// Moves the `row` column values to `new_table`, for the columns shared between both tables.
    /// Returns the index of the new row in `new_table` and the entity in this table swapped in
    /// to replace it (if an entity was swapped in).
    ///
    /// # Safety
    /// row must be in-bounds
    pub(crate) unsafe fn move_to_and_drop_missing_unchecked(
        &mut self,
        row: TableRow,
        new_table: &mut Table,
    ) -> TableMoveResult {
        debug_assert!(row.as_usize() < self.entity_count());
        let is_last = row.as_usize() == self.entities.len() - 1;
        let new_row = new_table.allocate(self.entities.swap_remove(row.as_usize()));
        for (component_id, column) in self.columns.iter_mut() {
            if let Some(new_column) = new_table.get_column_mut(*component_id) {
                new_column.initialize_from_unchecked(column, row, new_row);
            } else {
                column.swap_remove_unchecked(row);
            }
        }
        TableMoveResult {
            new_row,
            swapped_entity: if is_last {
                None
            } else {
                Some(self.entities[row.as_usize()])
            },
        }
    }

    /// Moves the `row` column values to `new_table`, for the columns shared between both tables.
    /// Returns the index of the new row in `new_table` and the entity in this table swapped in
    /// to replace it (if an entity was swapped in).
    ///
    /// # Safety
    /// `row` must be in-bounds. `new_table` must contain every component this table has
    pub(crate) unsafe fn move_to_superset_unchecked(
        &mut self,
        row: TableRow,
        new_table: &mut Table,
    ) -> TableMoveResult {
        debug_assert!(row.as_usize() < self.entity_count());
        let is_last = row.as_usize() == self.entities.len() - 1;
        let new_row = new_table.allocate(self.entities.swap_remove(row.as_usize()));
        for (component_id, column) in self.columns.iter_mut() {
            new_table
                .get_column_mut(*component_id)
                .debug_checked_unwrap()
                .initialize_from_unchecked(column, row, new_row);
        }
        TableMoveResult {
            new_row,
            swapped_entity: if is_last {
                None
            } else {
                Some(self.entities[row.as_usize()])
            },
        }
    }

    /// Fetches a read-only reference to the [`Column`] for a given [`Component`] within the
    /// table.
    ///
    /// Returns `None` if the corresponding component does not belong to the table.
    ///
    /// [`Component`]: crate::component::Component
    #[inline]
    pub fn get_column(&self, component_id: ComponentId) -> Option<&Column> {
        self.columns.get(component_id)
    }

    /// Fetches a mutable reference to the [`Column`] for a given [`Component`] within the
    /// table.
    ///
    /// Returns `None` if the corresponding component does not belong to the table.
    ///
    /// [`Component`]: crate::component::Component
    #[inline]
    pub(crate) fn get_column_mut(&mut self, component_id: ComponentId) -> Option<&mut Column> {
        self.columns.get_mut(component_id)
    }

    /// Checks if the table contains a [`Column`] for a given [`Component`].
    ///
    /// Returns `true` if the column is present, `false` otherwise.
    ///
    /// [`Component`]: crate::component::Component
    #[inline]
    pub fn has_column(&self, component_id: ComponentId) -> bool {
        self.columns.contains(component_id)
    }

    /// Reserves `additional` elements worth of capacity within the table.
    pub(crate) fn reserve(&mut self, additional: usize) {
        if self.entities.capacity() - self.entities.len() < additional {
            self.entities.reserve(additional);

            // use entities vector capacity as driving capacity for all related allocations
            let new_capacity = self.entities.capacity();

            for column in self.columns.values_mut() {
                column.reserve_exact(new_capacity - column.len());
            }
        }
    }

    /// Allocates space for a new entity
    ///
    /// # Safety
    /// the allocated row must be written to immediately with valid values in each column
    pub(crate) unsafe fn allocate(&mut self, entity: Entity) -> TableRow {
        self.reserve(1);
        let index = self.entities.len();
        self.entities.push(entity);
        for column in self.columns.values_mut() {
            column.data.set_len(self.entities.len());
            column.added_ticks.push(UnsafeCell::new(Tick::new(0)));
            column.changed_ticks.push(UnsafeCell::new(Tick::new(0)));
        }
        TableRow::from_usize(index)
    }

    /// Gets the number of entities currently being stored in the table.
    #[inline]
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Gets the number of components being stored in the table.
    #[inline]
    pub fn component_count(&self) -> usize {
        self.columns.len()
    }

    /// Gets the maximum number of entities the table can currently store
    /// without reallocating the underlying memory.
    #[inline]
    pub fn entity_capacity(&self) -> usize {
        self.entities.capacity()
    }

    /// Checks if the [`Table`] is empty or not.
    ///
    /// Returns `true` if the table contains no entities, `false` otherwise.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    pub(crate) fn check_change_ticks(&mut self, change_tick: Tick) {
        for column in self.columns.values_mut() {
            column.check_change_ticks(change_tick);
        }
    }

    /// Iterates over the [`Column`]s of the [`Table`].
    pub fn iter(&self) -> impl Iterator<Item = &Column> {
        self.columns.values()
    }

    /// Clears all of the stored components in the [`Table`].
    pub(crate) fn clear(&mut self) {
        self.entities.clear();
        for column in self.columns.values_mut() {
            column.clear();
        }
    }
}

/// A collection of [`Table`] storages, indexed by [`TableId`]
///
/// Can be accessed via [`Storages`](crate::storage::Storages)
pub struct Tables {
    tables: Vec<Table>,
    table_ids: HashMap<Box<[ComponentId]>, TableId>,
}

impl Default for Tables {
    fn default() -> Self {
        let empty_table = TableBuilder::with_capacity(0, 0).build();
        Tables {
            tables: vec![empty_table],
            table_ids: HashMap::default(),
        }
    }
}

pub(crate) struct TableMoveResult {
    pub swapped_entity: Option<Entity>,
    pub new_row: TableRow,
}

impl Tables {
    /// Returns the number of [`Table`]s this collection contains
    #[inline]
    pub fn len(&self) -> usize {
        self.tables.len()
    }

    /// Returns true if this collection contains no [`Table`]s
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.tables.is_empty()
    }

    /// Fetches a [`Table`] by its [`TableId`].
    ///
    /// Returns `None` if `id` is invalid.
    #[inline]
    pub fn get(&self, id: TableId) -> Option<&Table> {
        self.tables.get(id.as_usize())
    }

    /// Fetches mutable references to two different [`Table`]s.
    ///
    /// # Panics
    ///
    /// Panics if `a` and `b` are equal.
    #[inline]
    pub(crate) fn get_2_mut(&mut self, a: TableId, b: TableId) -> (&mut Table, &mut Table) {
        if a.as_usize() > b.as_usize() {
            let (b_slice, a_slice) = self.tables.split_at_mut(a.as_usize());
            (&mut a_slice[0], &mut b_slice[b.as_usize()])
        } else {
            let (a_slice, b_slice) = self.tables.split_at_mut(b.as_usize());
            (&mut a_slice[a.as_usize()], &mut b_slice[0])
        }
    }

    /// Attempts to fetch a table based on the provided components,
    /// creating and returning a new [`Table`] if one did not already exist.
    ///
    /// # Safety
    /// `component_ids` must contain components that exist in `components`
    pub(crate) unsafe fn get_id_or_insert(
        &mut self,
        component_ids: &[ComponentId],
        components: &Components,
    ) -> TableId {
        let tables = &mut self.tables;
        let (_key, value) = self
            .table_ids
            .raw_entry_mut()
            .from_key(component_ids)
            .or_insert_with(|| {
                let mut table = TableBuilder::with_capacity(0, component_ids.len());
                for component_id in component_ids {
                    table = table.add_column(components.get_info_unchecked(*component_id));
                }
                tables.push(table.build());
                (component_ids.into(), TableId::from_usize(tables.len() - 1))
            });

        *value
    }

    /// Iterates through all of the tables stored within in [`TableId`] order.
    pub fn iter(&self) -> std::slice::Iter<'_, Table> {
        self.tables.iter()
    }

    /// Clears all data from all [`Table`]s stored within.
    pub(crate) fn clear(&mut self) {
        for table in &mut self.tables {
            table.clear();
        }
    }

    pub(crate) fn check_change_ticks(&mut self, change_tick: Tick) {
        for table in &mut self.tables {
            table.check_change_ticks(change_tick);
        }
    }
}

impl Index<TableId> for Tables {
    type Output = Table;

    #[inline]
    fn index(&self, index: TableId) -> &Self::Output {
        &self.tables[index.as_usize()]
    }
}

impl IndexMut<TableId> for Tables {
    #[inline]
    fn index_mut(&mut self, index: TableId) -> &mut Self::Output {
        &mut self.tables[index.as_usize()]
    }
}

#[cfg(test)]
mod tests {
    use crate as bevy_ecs;
    use crate::component::Component;
    use crate::ptr::OwningPtr;
    use crate::storage::Storages;
    use crate::{
        component::{Components, Tick},
        entity::Entity,
        storage::{TableBuilder, TableRow},
    };
    #[derive(Component)]
    struct W<T>(T);

    #[test]
    fn table() {
        let mut components = Components::default();
        let mut storages = Storages::default();
        let component_id = components.init_component::<W<TableRow>>(&mut storages);
        let columns = &[component_id];
        let mut table = TableBuilder::with_capacity(0, columns.len())
            .add_column(components.get_info(component_id).unwrap())
            .build();
        let entities = (0..200).map(Entity::from_raw).collect::<Vec<_>>();
        for entity in &entities {
            // SAFETY: we allocate and immediately set data afterwards
            unsafe {
                let row = table.allocate(*entity);
                let value: W<TableRow> = W(row);
                OwningPtr::make(value, |value_ptr| {
                    table.get_column_mut(component_id).unwrap().initialize(
                        row,
                        value_ptr,
                        Tick::new(0),
                    );
                });
            };
        }

        assert_eq!(table.entity_capacity(), 256);
        assert_eq!(table.entity_count(), 200);
    }
}

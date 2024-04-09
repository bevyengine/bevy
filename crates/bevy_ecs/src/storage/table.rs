use crate::{
    component::{ComponentId, ComponentInfo, ComponentTicks, Components, Tick},
    entity::Entity,
    query::DebugCheckedUnwrap,
    storage::{blob_vec::BlobVec, ImmutableSparseSet, SparseSet},
};
use bevy_ptr::{OwningPtr, Ptr, UnsafeCellDeref};
use bevy_utils::HashMap;
pub(crate) use column::*;
use std::{alloc::Layout, num::NonZeroUsize, ptr::NonNull};
use std::{
    cell::UnsafeCell,
    ops::{Index, IndexMut},
};
mod column;

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

// TODO: Docs
/// A builder type for constructing [`Table`]s.
///
///  - Use [`with_capacity`] to initialize the builder.
///  - Repeatedly call [`add_column`] to add columns for components.
///  - Finalize with [`build`] to get the constructed [`Table`].
///
/// [`with_capacity`]: Self::with_capacity
/// [`add_column`]: Self::add_column
/// [`build`]: Self::build
pub struct TableBuilder {
    columns: SparseSet<ComponentId, ThinColumn<false>>,
    zst_columns: SparseSet<ComponentId, ThinColumn<true>>,
    capacity: usize,
}

impl TableBuilder {
    ///  Start building a new [`Table`] with a specified `column_capacity` (How many components per column?) and a `capacity` (How many columns?)
    pub fn with_capacity(capacity: usize, column_capacity: usize) -> Self {
        Self {
            columns: SparseSet::with_capacity(column_capacity),
            zst_columns: SparseSet::with_capacity(column_capacity),
            capacity,
        }
    }

    /// Add a new column to the [`Table`]. Specify the component which will be stored in the [`column`](ThinColumn) using its [`ComponentId`]
    #[must_use]
    pub fn add_column(mut self, component_info: &ComponentInfo) -> Self {
        match column_with_capacity(self.capacity, component_info) {
            ColumnCreationResult::ZST(col) => self.zst_columns.insert(component_info.id(), col),
            ColumnCreationResult::NotZST(col) => self.columns.insert(component_info.id(), col),
        }
        self
    }

    /// Build the [`Table`], after this operation the caller wouldn't be able to add more columns. The [`Table`] will be ready to use.
    #[must_use]
    pub fn build(self) -> Table {
        Table {
            columns: self.columns.into_immutable(),
            zst_columns: self.zst_columns.into_immutable(),
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
    columns: ImmutableSparseSet<ComponentId, ThinColumn<false>>,
    zst_columns: ImmutableSparseSet<ComponentId, ThinColumn<true>>,
    entities: Vec<Entity>,
}

// TODO: Check all methods and make sure documentation is up to date
impl Table {
    /// Fetches a read-only slice of the entities stored within the [`Table`].
    #[inline]
    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }

    /// Get the capacity of this table, this is equivelant to `self.entities.capacity()`
    /// Note that if an allocation is in process, this might not match the actual capacity of the columns, but it should once the allocation ends.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.entities.capacity()
    }

    /// Get the length of this table, this is equivelant to `self.entities.len()` or [`Self::entity_count`]
    #[inline]
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Removes the entity at the given row and returns the entity swapped in to replace it (if an
    /// entity was swapped in)
    ///
    /// # Safety
    /// `row` must be in-bounds (`row` < `len`)
    pub(crate) unsafe fn swap_remove_unchecked(&mut self, row: TableRow) -> Option<Entity> {
        debug_assert!(row.as_usize() < self.len());
        let last_element_index = self.len() - 1;
        // SAFETY:
        // - `row` < `len`
        // - `last_element_index` = `len`
        // - the `len` is kept within `self.entities`, it will update accordingly.
        for col in self.columns.values_mut() {
            unsafe { col.swap_remove_and_drop_unchecked(last_element_index, row) };
        }
        for zst_col in self.zst_columns.values_mut() {
            unsafe { zst_col.swap_remove_and_drop_unchecked(last_element_index, row) };
        }
        let is_last = row.as_usize() == last_element_index;
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
        let last_element_index = self.len() - 1;
        let is_last = row.as_usize() == last_element_index;
        let new_row = new_table.allocate(self.entities.swap_remove(row.as_usize()));
        for (component_id, column) in self.columns.iter_mut() {
            if let Some(new_column) = new_table.get_thin_column_mut(*component_id) {
                new_column.initialize_from_unchecked(column, last_element_index, row, new_row);
            } else {
                // It's the caller's responsibility to drop these cases.
                column.swap_remove_and_forget_unchecked(last_element_index, row);
            }
        }
        for (component_id, zst_column) in self.zst_columns.iter_mut() {
            if let Some(new_column) = new_table.get_thin_zst_column_mut(*component_id) {
                new_column.initialize_from_unchecked(zst_column, last_element_index, row, new_row);
            } else {
                // It's the caller's responsibility to drop these cases.
                zst_column.swap_remove_and_forget_unchecked(last_element_index, row);
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
        let last_element_index = self.len() - 1;
        let is_last = row.as_usize() == last_element_index;
        let new_row = new_table.allocate(self.entities.swap_remove(row.as_usize()));
        for (component_id, column) in self.columns.iter_mut() {
            if let Some(new_column) = new_table.get_thin_column_mut(*component_id) {
                new_column.initialize_from_unchecked(column, last_element_index, row, new_row);
            } else {
                column.swap_remove_and_drop_unchecked(last_element_index, row);
            }
        }
        for (component_id, zst_column) in self.zst_columns.iter_mut() {
            if let Some(new_column) = new_table.get_thin_zst_column_mut(*component_id) {
                new_column.initialize_from_unchecked(zst_column, last_element_index, row, new_row);
            } else {
                zst_column.swap_remove_and_drop_unchecked(last_element_index, row);
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

    // TODO: Docs
    ///
    pub unsafe fn initialize_component(
        &mut self,
        row: TableRow,
        component_id: ComponentId,
        comp_ptr: OwningPtr<'_>,
        change_tick: Tick,
    ) {
        if let Some(col) = self.get_thin_column_mut(component_id) {
            col.initialize(row, comp_ptr, change_tick)
        }
    }

    // TODO: Docs
    ///
    pub unsafe fn replace_component(
        &mut self,
        row: TableRow,
        component_id: ComponentId,
        comp_ptr: OwningPtr<'_>,
        change_tick: Tick,
    ) {
        if let Some(col) = self.get_thin_column_mut(component_id) {
            col.replace(row, comp_ptr, change_tick)
        }
    }

    /// Moves the `row` column values to `new_table`, for the columns shared between both tables.
    /// Returns the index of the new row in `new_table` and the entity in this table swapped in
    /// to replace it (if an entity was swapped in).
    ///
    /// # Safety
    /// - `row` must be in-bounds
    /// - `new_table` must contain every component this table has
    pub(crate) unsafe fn move_to_superset_unchecked(
        &mut self,
        row: TableRow,
        new_table: &mut Table,
    ) -> TableMoveResult {
        debug_assert!(row.as_usize() < self.entity_count());
        let last_element_index = self.len() - 1;
        let is_last = row.as_usize() == last_element_index;
        let new_row = new_table.allocate(self.entities.swap_remove(row.as_usize()));
        for (component_id, column) in self.columns.iter_mut() {
            new_table
                .get_thin_column_mut(*component_id)
                .debug_checked_unwrap()
                .initialize_from_unchecked(column, last_element_index, row, new_row);
        }
        for (component_id, zst_column) in self.zst_columns.iter_mut() {
            new_table
                .get_thin_zst_column_mut(*component_id)
                .debug_checked_unwrap()
                .initialize_from_unchecked(zst_column, last_element_index, row, new_row);
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

    // TODO: Docs
    /// # Safety
    /// - `T` must match the `component_id`
    pub unsafe fn get_column_data_slice<T>(
        &self,
        component_id: ComponentId,
    ) -> Option<&[UnsafeCell<T>]> {
        if let Some(col) = self.get_thin_column(component_id) {
            return Some(col.data.get_sub_slice(self.len()));
        }
        self.get_thin_zst_column(component_id)
            .map(|col| col.data.get_sub_slice(self.len()))
    }

    // TODO: Docs
    /// # Safety
    /// - `T` must match the `component_id`
    pub unsafe fn get_column_added_ticks(
        &self,
        component_id: ComponentId,
    ) -> Option<&[UnsafeCell<Tick>]> {
        if let Some(col) = self.get_thin_column(component_id) {
            return Some(col.added_ticks.to_slice(self.len()));
        }
        self.get_thin_zst_column(component_id)
            .map(|col| col.added_ticks.to_slice(self.len()))
    }

    // TODO: Docs
    /// # Safety
    /// - `T` must match the `component_id`
    pub unsafe fn get_column_changed_ticks(
        &self,
        component_id: ComponentId,
    ) -> Option<&[UnsafeCell<Tick>]> {
        if let Some(col) = self.get_thin_column(component_id) {
            return Some(col.changed_ticks.to_slice(self.len()));
        }
        self.get_thin_zst_column(component_id)
            .map(|col| col.changed_ticks.to_slice(self.len()))
    }

    // TODO: Docs
    /// # Safety
    /// - `T` must match the `component_id`
    pub unsafe fn get_column_changed_tick(
        &self,
        component_id: ComponentId,
        row: TableRow,
    ) -> &UnsafeCell<Tick> {
        if let Some(col) = self.get_thin_column(component_id) {
            return col.changed_ticks.get_unchecked(row.as_usize());
        }
        self.get_thin_zst_column(component_id)
            .debug_checked_unwrap()
            .changed_ticks
            .get_unchecked(row.as_usize())
    }

    // TODO: Docs
    /// # Safety
    /// - `T` must match the `component_id`
    pub unsafe fn get_column_added_tick(
        &self,
        component_id: ComponentId,
        row: TableRow,
    ) -> &UnsafeCell<Tick> {
        if let Some(col) = self.get_thin_column(component_id) {
            return col.added_ticks.get_unchecked(row.as_usize());
        }
        self.get_thin_zst_column(component_id)
            .debug_checked_unwrap()
            .added_ticks
            .get_unchecked(row.as_usize())
    }

    // TODO: Docs
    ///
    pub unsafe fn get_ticks_unchecked(
        &self,
        component_id: ComponentId,
        row: TableRow,
    ) -> Option<ComponentTicks> {
        if let Some(col) = self.get_thin_column(component_id) {
            return Some(ComponentTicks {
                added: col.added_ticks.get_unchecked(row.as_usize()).read(),
                changed: col.changed_ticks.get_unchecked(row.as_usize()).read(),
            });
        }
        self.get_thin_zst_column(component_id)
            .map(|col| ComponentTicks {
                added: col.added_ticks.get_unchecked(row.as_usize()).read(),
                changed: col.changed_ticks.get_unchecked(row.as_usize()).read(),
            })
    }

    /// Fetches a read-only reference to the [`Column`] for a given [`Component`] within the
    /// table.
    ///
    /// Returns `None` if the corresponding component does not belong to the table.
    ///
    /// [`Component`]: crate::component::Component
    #[inline]
    pub fn get_thin_column(&self, component_id: ComponentId) -> Option<&ThinColumn<false>> {
        self.columns.get(component_id)
    }

    /// Fetches a mutable reference to the [`Column`] for a given [`Component`] within the
    /// table.
    ///
    /// Returns `None` if the corresponding component does not belong to the table.
    ///
    /// [`Component`]: crate::component::Component
    #[inline]
    pub(crate) fn get_thin_column_mut(
        &mut self,
        component_id: ComponentId,
    ) -> Option<&mut ThinColumn<false>> {
        self.columns.get_mut(component_id)
    }

    /// Fetches a read-only reference to the [`Column`] for a given [`Component`] within the
    /// table.
    ///
    /// Returns `None` if the corresponding component does not belong to the table.
    ///
    /// [`Component`]: crate::component::Component
    #[inline]
    pub fn get_thin_zst_column(&self, component_id: ComponentId) -> Option<&ThinColumn<true>> {
        self.zst_columns.get(component_id)
    }

    /// Fetches a mutable reference to the [`Column`] for a given [`Component`] within the
    /// table.
    ///
    /// Returns `None` if the corresponding component does not belong to the table.
    ///
    /// [`Component`]: crate::component::Component
    #[inline]
    pub(crate) fn get_thin_zst_column_mut(
        &mut self,
        component_id: ComponentId,
    ) -> Option<&mut ThinColumn<true>> {
        self.zst_columns.get_mut(component_id)
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
        if self.capacity() - self.len() < additional {
            let column_cap = self.capacity();
            self.entities.reserve(additional);

            // use entities vector capacity as driving capacity for all related allocations
            let new_capacity = self.entities.capacity();

            if column_cap == 0 {
                unsafe { self.alloc_columns(NonZeroUsize::new_unchecked(new_capacity)) };
            } else {
                // SAFETY:
                // - `column_cap` is indeed the columns' capacity
                // - 0 < `additional` <= `self.len() + additional` <= `new_capacity`
                unsafe {
                    self.realloc_columns(
                        NonZeroUsize::new_unchecked(column_cap),
                        NonZeroUsize::new_unchecked(new_capacity),
                    )
                };
            }
        }
    }

    /// # Safety
    /// - The current capacity of the columns in 0
    pub(crate) unsafe fn alloc_columns(&mut self, new_capacity: NonZeroUsize) {
        for col in self.columns.values_mut() {
            col.alloc(new_capacity);
        }
        for zst_col in self.zst_columns.values_mut() {
            zst_col.alloc(new_capacity);
        }
    }

    /// # Safety
    /// - `current_column_capacity` is indeed the capacity of the columns
    pub(crate) unsafe fn realloc_columns(
        &mut self,
        current_column_capacity: NonZeroUsize,
        new_capacity: NonZeroUsize,
    ) {
        // SAFETY:
        // - There's no overflow
        // - `current_capacity` is indeed the capacity - safety requirement
        // - current capacity > 0
        for col in self.columns.values_mut() {
            col.realloc(current_column_capacity, new_capacity);
        }
        for zst_col in self.zst_columns.values_mut() {
            zst_col.realloc(current_column_capacity, new_capacity);
        }
    }

    /// Allocates space for a new entity
    ///
    /// # Safety
    /// the allocated row must be written to immediately with valid values in each column
    pub(crate) unsafe fn allocate(&mut self, entity: Entity) -> TableRow {
        self.reserve(1);
        let len = self.len();
        let cap = self.capacity();
        self.entities.push(entity);
        for col in self.columns.values_mut() {
            col.added_ticks
                .push(cap, len, UnsafeCell::new(Tick::new(0)));
            col.changed_ticks
                .push(cap, len, UnsafeCell::new(Tick::new(0)));
        }
        for zst_col in self.zst_columns.values_mut() {
            zst_col
                .added_ticks
                .push(cap, len, UnsafeCell::new(Tick::new(0)));
            zst_col
                .changed_ticks
                .push(cap, len, UnsafeCell::new(Tick::new(0)));
        }
        TableRow::from_usize(len)
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
        let len = self.len();
        for col in self.columns.values_mut() {
            // SAFETY: `len` is the actual length of the column
            unsafe { col.check_change_ticks(len, change_tick) };
        }
        for zst_col in self.zst_columns.values_mut() {
            // SAFETY: `len` is the actual length of the column
            unsafe { zst_col.check_change_ticks(len, change_tick) };
        }
    }

    /// Iterates over the [`Column`]s of the [`Table`].
    pub fn iter_columns(&self) -> impl Iterator<Item = &ThinColumn<false>> {
        self.columns.values()
    }

    /// Iterates over the ZST [`Column`]s of the [`Table`].
    pub fn iter_zst_columns(&self) -> impl Iterator<Item = &ThinColumn<true>> {
        self.zst_columns.values()
    }

    /// Clears all of the stored components in the [`Table`].
    pub(crate) fn clear(&mut self) {
        let len = self.len();
        // SAFETY: we defer `self.entities.clear()` until after clearing the columns, so `self.len()` should match the columns' len
        for column in self.columns.values_mut() {
            unsafe { column.clear(len) };
        }
        for zst_column in self.zst_columns.values_mut() {
            unsafe { zst_column.clear(len) };
        }
        self.entities.clear();
    }

    /// Moves component data out of the [`Table`].
    ///
    /// This function leaves the underlying memory unchanged, but the component behind
    /// returned pointer is semantically owned by the caller and will not be dropped in its original location.
    /// Caller is responsible to drop component data behind returned pointer.
    ///
    /// # Safety
    /// - this table must hold the component matching `component_id`
    /// - `row` must be in bounds
    pub unsafe fn take_component(
        &mut self,
        component_id: ComponentId,
        row: TableRow,
    ) -> OwningPtr<'_> {
        if let Some(col) = self.get_thin_column_mut(component_id) {
            return col.data.get_unchecked_mut(row.as_usize()).promote();
        }
        // TODO: Is this actually safe? We're need to return a pointer to a ZST - but this is no different than if we actually fetched the data, right?
        OwningPtr::new(NonNull::dangling())
    }

    // TODO: Docs
    ///
    pub unsafe fn get_component(
        &self,
        component_id: ComponentId,
        row: TableRow,
    ) -> Option<Ptr<'_>> {
        if let Some(col) = self.get_thin_column(component_id) {
            return Some(col.data.get_unchecked(row.as_usize()));
        }
        self.get_thin_zst_column(component_id)
            .map(|col| col.data.get_unchecked(row.as_usize()))
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

impl Drop for Table {
    fn drop(&mut self) {
        let len = self.len();
        let cap = self.capacity();
        for col in self.columns.values_mut() {
            // SAFETY: `cap` and `len` are correct
            unsafe {
                col.drop(cap, len);
            }
        }
        for col in self.zst_columns.values_mut() {
            // SAFETY: `cap` and `len` are correct
            unsafe {
                col.drop(cap, len);
            }
        }
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
                    table.get_thin_column_mut(component_id).unwrap().initialize(
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

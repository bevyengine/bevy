use crate::{
    component::{ComponentId, ComponentInfo, ComponentTicks, Components},
    entity::Entity,
    query::DebugCheckedUnwrap,
    storage::{blob_vec::BlobVec, ImmutableSparseSet, SparseSet},
};
use bevy_ptr::{OwningPtr, Ptr, PtrMut};
use bevy_utils::HashMap;
use std::alloc::Layout;
use std::{
    cell::UnsafeCell,
    ops::{Index, IndexMut},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableId(usize);

impl TableId {
    #[inline]
    pub fn new(index: usize) -> Self {
        TableId(index)
    }

    #[inline]
    pub fn index(self) -> usize {
        self.0
    }

    #[inline]
    pub const fn empty() -> TableId {
        TableId(0)
    }
}

#[derive(Debug)]
pub struct Column {
    data: BlobVec,
    ticks: Vec<UnsafeCell<ComponentTicks>>,
}

impl Column {
    #[inline]
    pub(crate) fn with_capacity(component_info: &ComponentInfo, capacity: usize) -> Self {
        Column {
            // SAFETY: component_info.drop() is valid for the types that will be inserted.
            data: unsafe { BlobVec::new(component_info.layout(), component_info.drop(), capacity) },
            ticks: Vec::with_capacity(capacity),
        }
    }

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
    pub(crate) unsafe fn initialize(
        &mut self,
        row: usize,
        data: OwningPtr<'_>,
        ticks: ComponentTicks,
    ) {
        debug_assert!(row < self.len());
        self.data.initialize_unchecked(row, data);
        *self.ticks.get_unchecked_mut(row).get_mut() = ticks;
    }

    /// Writes component data to the column at given row.
    /// Assumes the slot is initialized, calls drop.
    ///
    /// # Safety
    /// Assumes data has already been allocated for the given row.
    #[inline]
    pub(crate) unsafe fn replace(&mut self, row: usize, data: OwningPtr<'_>, change_tick: u32) {
        debug_assert!(row < self.len());
        self.data.replace_unchecked(row, data);
        self.ticks
            .get_unchecked_mut(row)
            .get_mut()
            .set_changed(change_tick);
    }

    /// Writes component data to the column at given row.
    /// Assumes the slot is initialized, calls drop.
    /// Does not update the Component's ticks.
    ///
    /// # Safety
    /// Assumes data has already been allocated for the given row.
    #[inline]
    pub(crate) unsafe fn replace_untracked(&mut self, row: usize, data: OwningPtr<'_>) {
        debug_assert!(row < self.len());
        self.data.replace_unchecked(row, data);
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// # Safety
    /// index must be in-bounds
    #[inline]
    pub(crate) unsafe fn swap_remove_unchecked(&mut self, row: usize) {
        self.data.swap_remove_and_drop_unchecked(row);
        self.ticks.swap_remove(row);
    }

    #[inline]
    #[must_use = "The returned pointer should be used to drop the removed component"]
    pub(crate) fn swap_remove_and_forget(
        &mut self,
        row: usize,
    ) -> Option<(OwningPtr<'_>, ComponentTicks)> {
        (row < self.data.len()).then(|| {
            // SAFETY: The row was length checked before this.
            let data = unsafe { self.data.swap_remove_and_forget_unchecked(row) };
            let ticks = self.ticks.swap_remove(row).into_inner();
            (data, ticks)
        })
    }

    /// # Safety
    /// index must be in-bounds
    #[inline]
    #[must_use = "The returned pointer should be used to dropped the removed component"]
    pub(crate) unsafe fn swap_remove_and_forget_unchecked(
        &mut self,
        row: usize,
    ) -> (OwningPtr<'_>, ComponentTicks) {
        let data = self.data.swap_remove_and_forget_unchecked(row);
        let ticks = self.ticks.swap_remove(row).into_inner();
        (data, ticks)
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
        src_row: usize,
        dst_row: usize,
    ) {
        debug_assert!(self.data.layout() == other.data.layout());
        let ptr = self.data.get_unchecked_mut(dst_row);
        other.data.swap_remove_unchecked(src_row, ptr);
        *self.ticks.get_unchecked_mut(dst_row) = other.ticks.swap_remove(src_row);
    }

    // # Safety
    // - ptr must point to valid data of this column's component type
    pub(crate) unsafe fn push(&mut self, ptr: OwningPtr<'_>, ticks: ComponentTicks) {
        self.data.push(ptr);
        self.ticks.push(UnsafeCell::new(ticks));
    }

    #[inline]
    pub(crate) fn reserve_exact(&mut self, additional: usize) {
        self.data.reserve_exact(additional);
        self.ticks.reserve_exact(additional);
    }

    #[inline]
    pub fn get_data_ptr(&self) -> Ptr<'_> {
        self.data.get_ptr()
    }

    /// # Safety
    /// The type `T` must be the type of the items in this column.
    pub unsafe fn get_data_slice<T>(&self) -> &[UnsafeCell<T>] {
        self.data.get_slice()
    }

    #[inline]
    pub fn get_ticks_slice(&self) -> &[UnsafeCell<ComponentTicks>] {
        &self.ticks
    }

    #[inline]
    pub fn get(&self, row: usize) -> Option<(Ptr<'_>, &UnsafeCell<ComponentTicks>)> {
        (row < self.data.len())
            // SAFETY: The row is length checked before fetching the pointer. This is being
            // accessed through a read-only reference to the column.
            .then(|| unsafe { (self.data.get_unchecked(row), self.ticks.get_unchecked(row)) })
    }

    #[inline]
    pub fn get_data(&self, row: usize) -> Option<Ptr<'_>> {
        // SAFETY: The row is length checked before fetching the pointer. This is being
        // accessed through a read-only reference to the column.
        (row < self.data.len()).then(|| unsafe { self.data.get_unchecked(row) })
    }

    /// # Safety
    /// - index must be in-bounds
    /// - no other reference to the data of the same row can exist at the same time
    #[inline]
    pub unsafe fn get_data_unchecked(&self, row: usize) -> Ptr<'_> {
        debug_assert!(row < self.data.len());
        self.data.get_unchecked(row)
    }

    #[inline]
    pub fn get_data_mut(&mut self, row: usize) -> Option<PtrMut<'_>> {
        // SAFETY: The row is length checked before fetching the pointer. This is being
        // accessed through an exclusive reference to the column.
        (row < self.data.len()).then(|| unsafe { self.data.get_unchecked_mut(row) })
    }

    /// # Safety
    /// - index must be in-bounds
    /// - no other reference to the data of the same row can exist at the same time
    #[inline]
    pub(crate) unsafe fn get_data_unchecked_mut(&mut self, row: usize) -> PtrMut<'_> {
        debug_assert!(row < self.data.len());
        self.data.get_unchecked_mut(row)
    }

    #[inline]
    pub fn get_ticks(&self, row: usize) -> Option<&UnsafeCell<ComponentTicks>> {
        self.ticks.get(row)
    }

    /// # Safety
    /// index must be in-bounds
    #[inline]
    pub unsafe fn get_ticks_unchecked(&self, row: usize) -> &UnsafeCell<ComponentTicks> {
        debug_assert!(row < self.ticks.len());
        self.ticks.get_unchecked(row)
    }

    pub fn clear(&mut self) {
        self.data.clear();
        self.ticks.clear();
    }

    #[inline]
    pub(crate) fn check_change_ticks(&mut self, change_tick: u32) {
        for component_ticks in &mut self.ticks {
            component_ticks.get_mut().check_ticks(change_tick);
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

    pub fn add_column(&mut self, component_info: &ComponentInfo) {
        self.columns.insert(
            component_info.id(),
            Column::with_capacity(component_info, self.capacity),
        );
    }

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
/// each `Column` is a type-erased `Vec<T: Component>`. Each row corresponds to a single entity
/// (i.e. index 3 in Column A and index 3 in Column B point to different components on the same
/// entity). Fetching components from a table involves fetching the associated column for a
/// component type (via it's [`ComponentId`]), then fetching the entity's row within that column.
///
/// [structure-of-arrays]: https://en.wikipedia.org/wiki/AoS_and_SoA#Structure_of_arrays
/// [`Component`]: crate::component::Component
/// [`World`]: crate::world::World
pub struct Table {
    columns: ImmutableSparseSet<ComponentId, Column>,
    entities: Vec<Entity>,
}

impl Table {
    #[inline]
    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }

    /// Removes the entity at the given row and returns the entity swapped in to replace it (if an
    /// entity was swapped in)
    ///
    /// # Safety
    /// `row` must be in-bounds
    pub(crate) unsafe fn swap_remove_unchecked(&mut self, row: usize) -> Option<Entity> {
        for column in self.columns.values_mut() {
            column.swap_remove_unchecked(row);
        }
        let is_last = row == self.entities.len() - 1;
        self.entities.swap_remove(row);
        if is_last {
            None
        } else {
            Some(self.entities[row])
        }
    }

    /// Moves the `row` column values to `new_table`, for the columns shared between both tables.
    /// Returns the index of the new row in `new_table` and the entity in this table swapped in
    /// to replace it (if an entity was swapped in). missing columns will be "forgotten". It is
    /// the caller's responsibility to drop them
    ///
    /// # Safety
    /// Row must be in-bounds
    pub(crate) unsafe fn move_to_and_forget_missing_unchecked(
        &mut self,
        row: usize,
        new_table: &mut Table,
    ) -> TableMoveResult {
        debug_assert!(row < self.entity_count());
        let is_last = row == self.entities.len() - 1;
        let new_row = new_table.allocate(self.entities.swap_remove(row));
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
                Some(self.entities[row])
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
        row: usize,
        new_table: &mut Table,
    ) -> TableMoveResult {
        debug_assert!(row < self.entity_count());
        let is_last = row == self.entities.len() - 1;
        let new_row = new_table.allocate(self.entities.swap_remove(row));
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
                Some(self.entities[row])
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
        row: usize,
        new_table: &mut Table,
    ) -> TableMoveResult {
        debug_assert!(row < self.entity_count());
        let is_last = row == self.entities.len() - 1;
        let new_row = new_table.allocate(self.entities.swap_remove(row));
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
                Some(self.entities[row])
            },
        }
    }

    #[inline]
    pub fn get_column(&self, component_id: ComponentId) -> Option<&Column> {
        self.columns.get(component_id)
    }

    #[inline]
    pub(crate) fn get_column_mut(&mut self, component_id: ComponentId) -> Option<&mut Column> {
        self.columns.get_mut(component_id)
    }

    #[inline]
    pub fn has_column(&self, component_id: ComponentId) -> bool {
        self.columns.contains(component_id)
    }

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
    pub(crate) unsafe fn allocate(&mut self, entity: Entity) -> usize {
        self.reserve(1);
        let index = self.entities.len();
        self.entities.push(entity);
        for column in self.columns.values_mut() {
            column.data.set_len(self.entities.len());
            column.ticks.push(UnsafeCell::new(ComponentTicks::new(0)));
        }
        index
    }

    #[inline]
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    #[inline]
    pub fn component_count(&self) -> usize {
        self.columns.len()
    }

    #[inline]
    pub fn entity_capacity(&self) -> usize {
        self.entities.capacity()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    pub(crate) fn check_change_ticks(&mut self, change_tick: u32) {
        for column in self.columns.values_mut() {
            column.check_change_ticks(change_tick);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Column> {
        self.columns.values()
    }

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
    table_ids: HashMap<Vec<ComponentId>, TableId>,
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
    pub new_row: usize,
}

impl Tables {
    #[inline]
    pub fn len(&self) -> usize {
        self.tables.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.tables.is_empty()
    }

    #[inline]
    pub fn get(&self, id: TableId) -> Option<&Table> {
        self.tables.get(id.index())
    }

    #[inline]
    pub(crate) fn get_2_mut(&mut self, a: TableId, b: TableId) -> (&mut Table, &mut Table) {
        if a.index() > b.index() {
            let (b_slice, a_slice) = self.tables.split_at_mut(a.index());
            (&mut a_slice[0], &mut b_slice[b.index()])
        } else {
            let (a_slice, b_slice) = self.tables.split_at_mut(b.index());
            (&mut a_slice[a.index()], &mut b_slice[0])
        }
    }

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
                    table.add_column(components.get_info_unchecked(*component_id));
                }
                tables.push(table.build());
                (component_ids.to_vec(), TableId(tables.len() - 1))
            });

        *value
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Table> {
        self.tables.iter()
    }

    pub(crate) fn clear(&mut self) {
        for table in &mut self.tables {
            table.clear();
        }
    }

    pub(crate) fn check_change_ticks(&mut self, change_tick: u32) {
        for table in &mut self.tables {
            table.check_change_ticks(change_tick);
        }
    }
}

impl Index<TableId> for Tables {
    type Output = Table;

    #[inline]
    fn index(&self, index: TableId) -> &Self::Output {
        &self.tables[index.index()]
    }
}

impl IndexMut<TableId> for Tables {
    #[inline]
    fn index_mut(&mut self, index: TableId) -> &mut Self::Output {
        &mut self.tables[index.index()]
    }
}

#[cfg(test)]
mod tests {
    use crate as bevy_ecs;
    use crate::component::Component;
    use crate::ptr::OwningPtr;
    use crate::storage::Storages;
    use crate::{
        component::{ComponentTicks, Components},
        entity::Entity,
        storage::TableBuilder,
    };
    #[derive(Component)]
    struct W<T>(T);

    #[test]
    fn table() {
        let mut components = Components::default();
        let mut storages = Storages::default();
        let component_id = components.init_component::<W<usize>>(&mut storages);
        let columns = &[component_id];
        let mut builder = TableBuilder::with_capacity(0, columns.len());
        builder.add_column(components.get_info(component_id).unwrap());
        let mut table = builder.build();
        let entities = (0..200).map(Entity::from_raw).collect::<Vec<_>>();
        for entity in &entities {
            // SAFETY: we allocate and immediately set data afterwards
            unsafe {
                let row = table.allocate(*entity);
                let value: W<usize> = W(row);
                OwningPtr::make(value, |value_ptr| {
                    table.get_column_mut(component_id).unwrap().initialize(
                        row,
                        value_ptr,
                        ComponentTicks::new(0),
                    );
                });
            };
        }

        assert_eq!(table.entity_capacity(), 256);
        assert_eq!(table.entity_count(), 200);
    }
}

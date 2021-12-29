use crate::{
    component::{ComponentId, ComponentInfo, ComponentTicks, Components},
    entity::Entity,
    storage::{BlobVec, SparseSet},
};
use bevy_utils::{AHasher, HashMap};
use std::{
    cell::UnsafeCell,
    hash::{Hash, Hasher},
    ops::{Index, IndexMut},
    ptr::NonNull,
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

pub struct Column {
    pub(crate) component_id: ComponentId,
    pub(crate) data: BlobVec,
    pub(crate) ticks: Vec<UnsafeCell<ComponentTicks>>,
}

impl Column {
    #[inline]
    pub fn with_capacity(component_info: &ComponentInfo, capacity: usize) -> Self {
        Column {
            component_id: component_info.id(),
            data: BlobVec::new(component_info.layout(), component_info.drop(), capacity),
            ticks: Vec::with_capacity(capacity),
        }
    }

    /// Writes component data to the column at given row.
    /// Assumes the slot is uninitialized, drop is not called.
    /// To overwrite existing initialized value, use `replace` instead.
    ///
    /// # Safety
    /// Assumes data has already been allocated for the given row.
    #[inline]
    pub unsafe fn initialize(&mut self, row: usize, data: *mut u8, ticks: ComponentTicks) {
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
    pub unsafe fn replace(&mut self, row: usize, data: *mut u8, change_tick: u32) {
        debug_assert!(row < self.len());
        self.data.replace_unchecked(row, data);
        self.ticks
            .get_unchecked_mut(row)
            .get_mut()
            .set_changed(change_tick);
    }

    /// # Safety
    /// Assumes data has already been allocated for the given row.
    #[inline]
    pub unsafe fn initialize_data(&mut self, row: usize, data: *mut u8) {
        debug_assert!(row < self.len());
        self.data.initialize_unchecked(row, data);
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
    pub unsafe fn get_ticks_unchecked_mut(&mut self, row: usize) -> &mut ComponentTicks {
        debug_assert!(row < self.len());
        self.ticks.get_unchecked_mut(row).get_mut()
    }

    /// # Safety
    /// index must be in-bounds
    #[inline]
    pub(crate) unsafe fn swap_remove_unchecked(&mut self, row: usize) {
        self.data.swap_remove_and_drop_unchecked(row);
        self.ticks.swap_remove(row);
    }

    #[inline]
    pub(crate) unsafe fn swap_remove_and_forget_unchecked(
        &mut self,
        row: usize,
    ) -> (*mut u8, ComponentTicks) {
        let data = self.data.swap_remove_and_forget_unchecked(row);
        let ticks = self.ticks.swap_remove(row).into_inner();
        (data, ticks)
    }

    // # Safety
    // - ptr must point to valid data of this column's component type
    pub(crate) unsafe fn push(&mut self, ptr: *mut u8, ticks: ComponentTicks) {
        let row = self.data.push_uninit();
        self.data.initialize_unchecked(row, ptr);
        self.ticks.push(UnsafeCell::new(ticks));
    }

    #[inline]
    pub(crate) fn reserve_exact(&mut self, additional: usize) {
        self.data.reserve_exact(additional);
        self.ticks.reserve_exact(additional);
    }

    /// # Safety
    /// must ensure rust mutability rules are not violated
    #[inline]
    pub unsafe fn get_data_ptr(&self) -> NonNull<u8> {
        self.data.get_ptr()
    }

    #[inline]
    pub fn get_ticks_ptr(&self) -> *const UnsafeCell<ComponentTicks> {
        self.ticks.as_ptr()
    }

    #[inline]
    pub fn get_ticks_const_ptr(&self) -> *const ComponentTicks {
        // cast is valid, because UnsafeCell is repr(transparent)
        self.get_ticks_ptr() as *const ComponentTicks
    }

    /// # Safety
    /// - index must be in-bounds
    /// - no other reference to the data of the same row can exist at the same time
    /// - pointer cannot be dereferenced after mutable reference to this `Column` was live
    #[inline]
    pub unsafe fn get_data_unchecked(&self, row: usize) -> *mut u8 {
        debug_assert!(row < self.data.len());
        self.data.get_unchecked(row)
    }

    /// # Safety
    /// index must be in-bounds
    #[inline]
    pub unsafe fn get_ticks_unchecked(&self, row: usize) -> &ComponentTicks {
        debug_assert!(row < self.ticks.len());
        &*self.ticks.get_unchecked(row).get()
    }

    /// # Safety
    /// - index must be in-bounds
    /// - no other reference to the ticks of the same row can exist at the same time
    /// - pointer cannot be dereferenced after mutable reference to this column was live
    #[inline]
    pub unsafe fn get_ticks_mut_ptr_unchecked(&self, row: usize) -> *mut ComponentTicks {
        debug_assert!(row < self.ticks.len());
        self.ticks.get_unchecked(row).get()
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

pub struct Table {
    columns: SparseSet<ComponentId, Column>,
    entities: Vec<Entity>,
}

impl Table {
    pub const fn new() -> Table {
        Self {
            columns: SparseSet::new(),
            entities: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize, column_capacity: usize) -> Table {
        Self {
            columns: SparseSet::with_capacity(column_capacity),
            entities: Vec::with_capacity(capacity),
        }
    }

    #[inline]
    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }

    pub fn add_column(&mut self, component_info: &ComponentInfo) {
        self.columns.insert(
            component_info.id(),
            Column::with_capacity(component_info, self.entities.capacity()),
        )
    }

    /// Removes the entity at the given row and returns the entity swapped in to replace it (if an
    /// entity was swapped in)
    ///
    /// # Safety
    /// `row` must be in-bounds
    pub unsafe fn swap_remove_unchecked(&mut self, row: usize) -> Option<Entity> {
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
    pub unsafe fn move_to_and_forget_missing_unchecked(
        &mut self,
        row: usize,
        new_table: &mut Table,
    ) -> TableMoveResult {
        debug_assert!(row < self.len());
        let is_last = row == self.entities.len() - 1;
        let new_row = new_table.allocate(self.entities.swap_remove(row));
        for column in self.columns.values_mut() {
            let (data, ticks) = column.swap_remove_and_forget_unchecked(row);
            if let Some(new_column) = new_table.get_column_mut(column.component_id) {
                new_column.initialize(new_row, data, ticks);
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
    pub unsafe fn move_to_and_drop_missing_unchecked(
        &mut self,
        row: usize,
        new_table: &mut Table,
    ) -> TableMoveResult {
        debug_assert!(row < self.len());
        let is_last = row == self.entities.len() - 1;
        let new_row = new_table.allocate(self.entities.swap_remove(row));
        for column in self.columns.values_mut() {
            if let Some(new_column) = new_table.get_column_mut(column.component_id) {
                let (data, ticks) = column.swap_remove_and_forget_unchecked(row);
                new_column.initialize(new_row, data, ticks);
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
    pub unsafe fn move_to_superset_unchecked(
        &mut self,
        row: usize,
        new_table: &mut Table,
    ) -> TableMoveResult {
        debug_assert!(row < self.len());
        let is_last = row == self.entities.len() - 1;
        let new_row = new_table.allocate(self.entities.swap_remove(row));
        for column in self.columns.values_mut() {
            let new_column = new_table.get_column_mut(column.component_id).unwrap();
            let (data, ticks) = column.swap_remove_and_forget_unchecked(row);
            new_column.initialize(new_row, data, ticks);
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
    pub fn get_column_mut(&mut self, component_id: ComponentId) -> Option<&mut Column> {
        self.columns.get_mut(component_id)
    }

    #[inline]
    pub fn has_column(&self, component_id: ComponentId) -> bool {
        self.columns.contains(component_id)
    }

    pub fn reserve(&mut self, additional: usize) {
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
    pub unsafe fn allocate(&mut self, entity: Entity) -> usize {
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
    pub fn capacity(&self) -> usize {
        self.entities.capacity()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.entities.len()
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

    pub fn clear(&mut self) {
        self.entities.clear();
        for column in self.columns.values_mut() {
            column.clear();
        }
    }
}

/// A collection of [Table] storages, indexed by [TableId]
///
/// Can be accessed via [Storages](crate::storage::Storages)
pub struct Tables {
    tables: Vec<Table>,
    table_ids: HashMap<u64, TableId>,
}

impl Default for Tables {
    fn default() -> Self {
        let empty_table = Table::with_capacity(0, 0);
        Tables {
            tables: vec![empty_table],
            table_ids: HashMap::default(),
        }
    }
}

pub struct TableMoveResult {
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
    pub fn get_mut(&mut self, id: TableId) -> Option<&mut Table> {
        self.tables.get_mut(id.index())
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
    pub unsafe fn get_id_or_insert(
        &mut self,
        component_ids: &[ComponentId],
        components: &Components,
    ) -> TableId {
        let mut hasher = AHasher::default();
        component_ids.hash(&mut hasher);
        let hash = hasher.finish();
        let tables = &mut self.tables;
        *self.table_ids.entry(hash).or_insert_with(move || {
            let mut table = Table::with_capacity(0, component_ids.len());
            for component_id in component_ids.iter() {
                table.add_column(components.get_info_unchecked(*component_id));
            }
            tables.push(table);
            TableId(tables.len() - 1)
        })
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Table> {
        self.tables.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, Table> {
        self.tables.iter_mut()
    }

    pub fn clear(&mut self) {
        for table in self.tables.iter_mut() {
            table.clear();
        }
    }

    pub(crate) fn check_change_ticks(&mut self, change_tick: u32) {
        for table in self.tables.iter_mut() {
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
    use crate::storage::Storages;
    use crate::{component::Components, entity::Entity, storage::Table};
    #[derive(Component)]
    struct W<T>(T);

    #[test]
    fn table() {
        let mut components = Components::default();
        let mut storages = Storages::default();
        let component_id = components.init_component::<W<usize>>(&mut storages);
        let columns = &[component_id];
        let mut table = Table::with_capacity(0, columns.len());
        table.add_column(components.get_info(component_id).unwrap());
        let entities = (0..200).map(Entity::new).collect::<Vec<_>>();
        for entity in entities.iter() {
            // SAFE: we allocate and immediately set data afterwards
            unsafe {
                let row = table.allocate(*entity);
                let mut value = row;
                let value_ptr = ((&mut value) as *mut usize).cast::<u8>();
                table
                    .get_column_mut(component_id)
                    .unwrap()
                    .initialize_data(row, value_ptr);
            };
        }

        assert_eq!(table.capacity(), 256);
        assert_eq!(table.len(), 200);
    }
}

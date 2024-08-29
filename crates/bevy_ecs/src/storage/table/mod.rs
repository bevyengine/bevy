use crate::{
    change_detection::MaybeLocation,
    component::{ComponentId, ComponentInfo, ComponentTicks, Components, Tick},
    entity::Entity,
    query::DebugCheckedUnwrap,
    storage::{blob_vec::BlobVec, ImmutableSparseSet, SparseSet},
};
use bevy_ptr::{OwningPtr, Ptr, UnsafeCellDeref};
use bevy_utils::HashMap;
pub use column::*;
#[cfg(feature = "track_change_detection")]
use std::panic::Location;
use std::{alloc::Layout, num::NonZeroUsize};
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

// /// A type-erased contiguous container for data of a homogeneous type.
// ///
// /// Conceptually, a [`Column`] is very similar to a type-erased `Vec<T>`.
// /// It also stores the change detection ticks for its components, kept in two separate
// /// contiguous buffers internally. An element shares its data across these buffers by using the
// /// same index (i.e. the entity at row 3 has its data at index 3 and its change detection ticks at
// /// index 3). A slice to these contiguous blocks of memory can be fetched
// /// via [`Column::get_data_slice`], [`Column::get_added_ticks_slice`], and
// /// [`Column::get_changed_ticks_slice`].
// ///
// /// Like many other low-level storage types, [`Column`] has a limited and highly unsafe
// /// interface. It's highly advised to use higher level types and their safe abstractions
// /// instead of working directly with [`Column`].
// #[derive(Debug)]
// pub struct Column {
//     data: BlobVec,
//     added_ticks: Vec<UnsafeCell<Tick>>,
//     changed_ticks: Vec<UnsafeCell<Tick>>,
//     #[cfg(feature = "track_change_detection")]
//     changed_by: Vec<UnsafeCell<&'static Location<'static>>>,
// }

// impl Column {
//     /// Constructs a new [`Column`], configured with a component's layout and an initial `capacity`.
//     #[inline]
//     pub(crate) fn with_capacity(component_info: &ComponentInfo, capacity: usize) -> Self {
//         Column {
//             // SAFETY: component_info.drop() is valid for the types that will be inserted.
//             data: unsafe { BlobVec::new(component_info.layout(), component_info.drop(), capacity) },
//             added_ticks: Vec::with_capacity(capacity),
//             changed_ticks: Vec::with_capacity(capacity),
//             #[cfg(feature = "track_change_detection")]
//             changed_by: Vec::with_capacity(capacity),
//         }
//     }

//     /// Fetches the [`Layout`] for the underlying type.
//     #[inline]
//     pub fn item_layout(&self) -> Layout {
//         self.data.layout()
//     }

//     /// Writes component data to the column at given row.
//     /// Assumes the slot is uninitialized, drop is not called.
//     /// To overwrite existing initialized value, use `replace` instead.
//     ///
//     /// # Safety
//     /// Assumes data has already been allocated for the given row.
//     #[inline]
//     pub(crate) unsafe fn initialize(
//         &mut self,
//         row: TableRow,
//         data: OwningPtr<'_>,
//         tick: Tick,
//         #[cfg(feature = "track_change_detection")] caller: &'static Location<'static>,
//     ) {
//         debug_assert!(row.as_usize() < self.len());
//         self.data.initialize_unchecked(row.as_usize(), data);
//         *self.added_ticks.get_unchecked_mut(row.as_usize()).get_mut() = tick;
//         *self
//             .changed_ticks
//             .get_unchecked_mut(row.as_usize())
//             .get_mut() = tick;
//         #[cfg(feature = "track_change_detection")]
//         {
//             *self.changed_by.get_unchecked_mut(row.as_usize()).get_mut() = caller;
//         }
//     }

//     /// Writes component data to the column at given row.
//     /// Assumes the slot is initialized, calls drop.
//     ///
//     /// # Safety
//     /// Assumes data has already been allocated for the given row.
//     #[inline]
//     pub(crate) unsafe fn replace(
//         &mut self,
//         row: TableRow,
//         data: OwningPtr<'_>,
//         change_tick: Tick,
//         #[cfg(feature = "track_change_detection")] caller: &'static Location<'static>,
//     ) {
//         debug_assert!(row.as_usize() < self.len());
//         self.data.replace_unchecked(row.as_usize(), data);
//         *self
//             .changed_ticks
//             .get_unchecked_mut(row.as_usize())
//             .get_mut() = change_tick;

//         #[cfg(feature = "track_change_detection")]
//         {
//             *self.changed_by.get_unchecked_mut(row.as_usize()).get_mut() = caller;
//         }
//     }

//     /// Call [`drop`] on a value.
//     ///
//     /// # Safety
//     /// `data` must point to the same type that this table stores, so the
//     /// correct drop function is called.
//     #[inline]
//     pub(crate) unsafe fn drop(&self, data: OwningPtr<'_>) {
//         if let Some(drop) = self.data.get_drop() {
//             // Safety: we're using the same drop fn that the BlobVec would
//             // if we inserted the data instead of dropping it.
//             unsafe { drop(data) }
//         }
//     }

//     /// Gets the current number of elements stored in the column.
//     #[inline]
//     pub fn len(&self) -> usize {
//         self.data.len()
//     }

//     /// Checks if the column is empty. Returns `true` if there are no elements, `false` otherwise.
//     #[inline]
//     pub fn is_empty(&self) -> bool {
//         self.data.is_empty()
//     }

//     /// Removes an element from the [`Column`].
//     ///
//     /// - The value will be dropped if it implements [`Drop`].
//     /// - This does not preserve ordering, but is O(1).
//     /// - This does not do any bounds checking.
//     /// - The element is replaced with the last element in the [`Column`].
//     ///
//     /// # Safety
//     /// `row` must be within the range `[0, self.len())`.
//     ///
//     #[inline]
//     pub(crate) unsafe fn swap_remove_unchecked(&mut self, row: TableRow) {
//         self.data.swap_remove_and_drop_unchecked(row.as_usize());
//         self.added_ticks.swap_remove(row.as_usize());
//         self.changed_ticks.swap_remove(row.as_usize());
//         #[cfg(feature = "track_change_detection")]
//         self.changed_by.swap_remove(row.as_usize());
//     }

//     /// Removes an element from the [`Column`] and returns it and its change detection ticks.
//     /// This does not preserve ordering, but is O(1) and does not do any bounds checking.
//     ///
//     /// The element is replaced with the last element in the [`Column`].
//     ///
//     /// It's the caller's responsibility to ensure that the removed value is dropped or used.
//     /// Failure to do so may result in resources not being released (i.e. files handles not being
//     /// released, memory leaks, etc.)
//     ///
//     /// # Safety
//     /// `row` must be within the range `[0, self.len())`.
//     #[inline]
//     #[must_use = "The returned pointer should be used to dropped the removed component"]
//     pub(crate) unsafe fn swap_remove_and_forget_unchecked(
//         &mut self,
//         row: TableRow,
//     ) -> (OwningPtr<'_>, ComponentTicks, MaybeLocation) {
//         let data = self.data.swap_remove_and_forget_unchecked(row.as_usize());
//         let added = self.added_ticks.swap_remove(row.as_usize()).into_inner();
//         let changed = self.changed_ticks.swap_remove(row.as_usize()).into_inner();
//         #[cfg(feature = "track_change_detection")]
//         let caller = self.changed_by.swap_remove(row.as_usize()).into_inner();
//         #[cfg(not(feature = "track_change_detection"))]
//         let caller = ();
//         (data, ComponentTicks { added, changed }, caller)
//     }

//     /// Pushes a new value onto the end of the [`Column`].
//     ///
//     /// # Safety
//     /// `ptr` must point to valid data of this column's component type
//     pub(crate) unsafe fn push(
//         &mut self,
//         ptr: OwningPtr<'_>,
//         ticks: ComponentTicks,
//         #[cfg(feature = "track_change_detection")] caller: &'static Location<'static>,
//     ) {
//         self.data.push(ptr);
//         self.added_ticks.push(UnsafeCell::new(ticks.added));
//         self.changed_ticks.push(UnsafeCell::new(ticks.changed));
//         #[cfg(feature = "track_change_detection")]
//         self.changed_by.push(UnsafeCell::new(caller));
//     }

//     #[inline]
//     pub(crate) fn reserve_exact(&mut self, additional: usize) {
//         self.data.reserve_exact(additional);
//         self.added_ticks.reserve_exact(additional);
//         self.changed_ticks.reserve_exact(additional);
//         #[cfg(feature = "track_change_detection")]
//         self.changed_by.reserve_exact(additional);
//     }

//     /// Fetches the data pointer to the first element of the [`Column`].
//     ///
//     /// The pointer is type erased, so using this function to fetch anything
//     /// other than the first element will require computing the offset using
//     /// [`Column::item_layout`].
//     #[inline]
//     pub fn get_data_ptr(&self) -> Ptr<'_> {
//         self.data.get_ptr()
//     }

//     /// Fetches the slice to the [`Column`]'s data cast to a given type.
//     ///
//     /// Note: The values stored within are [`UnsafeCell`].
//     /// Users of this API must ensure that accesses to each individual element
//     /// adhere to the safety invariants of [`UnsafeCell`].
//     ///
//     /// # Safety
//     /// The type `T` must be the type of the items in this column.
//     pub unsafe fn get_data_slice<T>(&self) -> &[UnsafeCell<T>] {
//         self.data.get_slice()
//     }

//     /// Fetches the slice to the [`Column`]'s "added" change detection ticks.
//     ///
//     /// Note: The values stored within are [`UnsafeCell`].
//     /// Users of this API must ensure that accesses to each individual element
//     /// adhere to the safety invariants of [`UnsafeCell`].
//     #[inline]
//     pub fn get_added_ticks_slice(&self) -> &[UnsafeCell<Tick>] {
//         &self.added_ticks
//     }

//     /// Fetches the slice to the [`Column`]'s "changed" change detection ticks.
//     ///
//     /// Note: The values stored within are [`UnsafeCell`].
//     /// Users of this API must ensure that accesses to each individual element
//     /// adhere to the safety invariants of [`UnsafeCell`].
//     #[inline]
//     pub fn get_changed_ticks_slice(&self) -> &[UnsafeCell<Tick>] {
//         &self.changed_ticks
//     }

//     /// Fetches the slice to the [`Column`]'s caller locations.
//     ///
//     /// Note: The values stored within are [`UnsafeCell`].
//     /// Users of this API must ensure that accesses to each individual element
//     /// adhere to the safety invariants of [`UnsafeCell`].
//     #[inline]
//     #[cfg(feature = "track_change_detection")]
//     pub fn get_changed_by_slice(&self) -> &[UnsafeCell<&'static Location<'static>>] {
//         &self.changed_by
//     }

//     /// Fetches a reference to the data and change detection ticks at `row`.
//     ///
//     /// Returns `None` if `row` is out of bounds.
//     #[inline]
//     pub fn get(
//         &self,
//         row: TableRow,
//     ) -> Option<(Ptr<'_>, TickCells<'_>, MaybeUnsafeCellLocation<'_>)> {
//         (row.as_usize() < self.data.len())
//             // SAFETY: The row is length checked before fetching the pointer. This is being
//             // accessed through a read-only reference to the column.
//             .then(|| unsafe {
//                 (
//                     self.data.get_unchecked(row.as_usize()),
//                     TickCells {
//                         added: self.added_ticks.get_unchecked(row.as_usize()),
//                         changed: self.changed_ticks.get_unchecked(row.as_usize()),
//                     },
//                     #[cfg(feature = "track_change_detection")]
//                     self.changed_by.get_unchecked(row.as_usize()),
//                     #[cfg(not(feature = "track_change_detection"))]
//                     (),
//                 )
//             })
//     }

//     /// Fetches a read-only reference to the data at `row`.
//     ///
//     /// Returns `None` if `row` is out of bounds.
//     #[inline]
//     pub fn get_data(&self, row: TableRow) -> Option<Ptr<'_>> {
//         (row.as_usize() < self.data.len()).then(|| {
//             // SAFETY: The row is length checked before fetching the pointer. This is being
//             // accessed through a read-only reference to the column.
//             unsafe { self.data.get_unchecked(row.as_usize()) }
//         })
//     }

//     /// Fetches a read-only reference to the data at `row`. Unlike [`Column::get`] this does not
//     /// do any bounds checking.
//     ///
//     /// # Safety
//     /// - `row` must be within the range `[0, self.len())`.
//     /// - no other mutable reference to the data of the same row can exist at the same time
//     #[inline]
//     pub unsafe fn get_data_unchecked(&self, row: TableRow) -> Ptr<'_> {
//         debug_assert!(row.as_usize() < self.data.len());
//         self.data.get_unchecked(row.as_usize())
//     }

//     /// Fetches a mutable reference to the data at `row`.
//     ///
//     /// Returns `None` if `row` is out of bounds.
//     #[inline]
//     pub fn get_data_mut(&mut self, row: TableRow) -> Option<PtrMut<'_>> {
//         (row.as_usize() < self.data.len()).then(|| {
//             // SAFETY: The row is length checked before fetching the pointer. This is being
//             // accessed through an exclusive reference to the column.
//             unsafe { self.data.get_unchecked_mut(row.as_usize()) }
//         })
//     }

//     /// Fetches a mutable reference to the data at `row`. Unlike [`Column::get_data_mut`] this does not
//     /// do any bounds checking.
//     ///
//     /// # Safety
//     /// - index must be in-bounds
//     /// - no other reference to the data of the same row can exist at the same time
//     #[inline]
//     pub(crate) unsafe fn get_data_unchecked_mut(&mut self, row: TableRow) -> PtrMut<'_> {
//         debug_assert!(row.as_usize() < self.data.len());
//         self.data.get_unchecked_mut(row.as_usize())
//     }

//     /// Fetches the "added" change detection tick for the value at `row`.
//     ///
//     /// Returns `None` if `row` is out of bounds.
//     ///
//     /// Note: The values stored within are [`UnsafeCell`].
//     /// Users of this API must ensure that accesses to each individual element
//     /// adhere to the safety invariants of [`UnsafeCell`].
//     #[inline]
//     pub fn get_added_tick(&self, row: TableRow) -> Option<&UnsafeCell<Tick>> {
//         self.added_ticks.get(row.as_usize())
//     }

//     /// Fetches the "changed" change detection tick for the value at `row`.
//     ///
//     /// Returns `None` if `row` is out of bounds.
//     ///
//     /// Note: The values stored within are [`UnsafeCell`].
//     /// Users of this API must ensure that accesses to each individual element
//     /// adhere to the safety invariants of [`UnsafeCell`].
//     #[inline]
//     pub fn get_changed_tick(&self, row: TableRow) -> Option<&UnsafeCell<Tick>> {
//         self.changed_ticks.get(row.as_usize())
//     }

//     /// Fetches the change detection ticks for the value at `row`.
//     ///
//     /// Returns `None` if `row` is out of bounds.
//     #[inline]
//     pub fn get_ticks(&self, row: TableRow) -> Option<ComponentTicks> {
//         if row.as_usize() < self.data.len() {
//             // SAFETY: The size of the column has already been checked.
//             Some(unsafe { self.get_ticks_unchecked(row) })
//         } else {
//             None
//         }
//     }

//     /// Fetches the "added" change detection tick for the value at `row`. Unlike [`Column::get_added_tick`]
//     /// this function does not do any bounds checking.
//     ///
//     /// # Safety
//     /// `row` must be within the range `[0, self.len())`.
//     #[inline]
//     pub unsafe fn get_added_tick_unchecked(&self, row: TableRow) -> &UnsafeCell<Tick> {
//         debug_assert!(row.as_usize() < self.added_ticks.len());
//         self.added_ticks.get_unchecked(row.as_usize())
//     }

//     /// Fetches the "changed" change detection tick for the value at `row`. Unlike [`Column::get_changed_tick`]
//     /// this function does not do any bounds checking.
//     ///
//     /// # Safety
//     /// `row` must be within the range `[0, self.len())`.
//     #[inline]
//     pub unsafe fn get_changed_tick_unchecked(&self, row: TableRow) -> &UnsafeCell<Tick> {
//         debug_assert!(row.as_usize() < self.changed_ticks.len());
//         self.changed_ticks.get_unchecked(row.as_usize())
//     }

//     /// Fetches the change detection ticks for the value at `row`. Unlike [`Column::get_ticks`]
//     /// this function does not do any bounds checking.
//     ///
//     /// # Safety
//     /// `row` must be within the range `[0, self.len())`.
//     #[inline]
//     pub unsafe fn get_ticks_unchecked(&self, row: TableRow) -> ComponentTicks {
//         debug_assert!(row.as_usize() < self.added_ticks.len());
//         debug_assert!(row.as_usize() < self.changed_ticks.len());
//         ComponentTicks {
//             added: self.added_ticks.get_unchecked(row.as_usize()).read(),
//             changed: self.changed_ticks.get_unchecked(row.as_usize()).read(),
//         }
//     }

//     /// Fetches the calling location that last changed the value at `row`.
//     ///
//     /// Returns `None` if `row` is out of bounds.
//     ///
//     /// Note: The values stored within are [`UnsafeCell`].
//     /// Users of this API must ensure that accesses to each individual element
//     /// adhere to the safety invariants of [`UnsafeCell`].
//     #[inline]
//     #[cfg(feature = "track_change_detection")]
//     pub fn get_changed_by(&self, row: TableRow) -> Option<&UnsafeCell<&'static Location<'static>>> {
//         self.changed_by.get(row.as_usize())
//     }

//     /// Fetches the calling location that last changed the value at `row`.
//     ///
//     /// Unlike [`Column::get_changed_by`] this function does not do any bounds checking.
//     ///
//     /// # Safety
//     /// `row` must be within the range `[0, self.len())`.
//     #[inline]
//     #[cfg(feature = "track_change_detection")]
//     pub unsafe fn get_changed_by_unchecked(
//         &self,
//         row: TableRow,
//     ) -> &UnsafeCell<&'static Location<'static>> {
//         debug_assert!(row.as_usize() < self.changed_by.len());
//         self.changed_by.get_unchecked(row.as_usize())
//     }

//     /// Clears the column, removing all values.
//     ///
//     /// Note that this function has no effect on the allocated capacity of the [`Column`]>
//     pub fn clear(&mut self) {
//         self.data.clear();
//         self.added_ticks.clear();
//         self.changed_ticks.clear();
//         #[cfg(feature = "track_change_detection")]
//         self.changed_by.clear();
//     }

//     #[inline]
//     pub(crate) fn check_change_ticks(&mut self, change_tick: Tick) {
//         for component_ticks in &mut self.added_ticks {
//             component_ticks.get_mut().check_tick(change_tick);
//         }
//         for component_ticks in &mut self.changed_ticks {
//             component_ticks.get_mut().check_tick(change_tick);
//         }
//     }
// }

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
    columns: SparseSet<ComponentId, ThinColumn>,
    capacity: usize,
}

impl TableBuilder {
    /// Start building a new [`Table`] with a specified `column_capacity` (How many components per column?) and a `capacity` (How many columns?)
    pub fn with_capacity(capacity: usize, column_capacity: usize) -> Self {
        Self {
            columns: SparseSet::with_capacity(column_capacity),
            capacity,
        }
    }

    /// Add a new column to the [`Table`]. Specify the component which will be stored in the [`column`](ThinColumn) using its [`ComponentId`]
    #[must_use]
    pub fn add_column(mut self, component_info: &ComponentInfo) -> Self {
        self.columns.insert(
            component_info.id(),
            ThinColumn::with_capacity(component_info, self.capacity),
        );
        self
    }

    /// Build the [`Table`], after this operation the caller wouldn't be able to add more columns. The [`Table`] will be ready to use.
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
/// each [`ThinColumn`] is a type-erased `Vec<T: Component>`. Each row corresponds to a single entity
/// (i.e. index 3 in Column A and index 3 in Column B point to different components on the same
/// entity). Fetching components from a table involves fetching the associated column for a
/// component type (via its [`ComponentId`]), then fetching the entity's row within that column.
///
/// [structure-of-arrays]: https://en.wikipedia.org/wiki/AoS_and_SoA#Structure_of_arrays
/// [`Component`]: crate::component::Component
/// [`World`]: crate::world::World
pub struct Table {
    columns: ImmutableSparseSet<ComponentId, ThinColumn>,
    entities: Vec<Entity>,
}

struct AbortOnPanic;

impl Drop for AbortOnPanic {
    fn drop(&mut self) {
        // Panicking while unwinding will force an abort.
        panic!("Aborting due to allocator error");
    }
}

impl Table {
    /// Fetches a read-only slice of the entities stored within the [`Table`].
    #[inline]
    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }

    /// Get the capacity of this table, in entities.
    /// Note that if an allocation is in process, this might not match the actual capacity of the columns, but it should once the allocation ends.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.entities.capacity()
    }

    /// Removes the entity at the given row and returns the entity swapped in to replace it (if an
    /// entity was swapped in)
    ///
    /// # Safety
    /// `row` must be in-bounds (`row.as_usize()` < `self.len()`)
    pub(crate) unsafe fn swap_remove_unchecked(&mut self, row: TableRow) -> Option<Entity> {
        debug_assert!(row.as_usize() < self.entity_count());
        let last_element_index = self.entity_count() - 1;
        if row.as_usize() != last_element_index {
            // Instead of checking this condition on every `swap_remove` call, we
            // check it here and use `swap_remove_nonoverlapping`.
            for col in self.columns.values_mut() {
                // SAFETY:
                // - `row` < `len`
                // - `last_element_index` = `len` - 1
                // - `row` != `last_element_index`
                // - the `len` is kept within `self.entities`, it will update accordingly.
                unsafe {
                    col.swap_remove_and_drop_unchecked_nonoverlapping(last_element_index, row);
                };
            }
        } else {
            // If `row.as_usize()` == `last_element_index` than there's no point in removing the component
            // at `row`, but we still need to drop it.
            for col in self.columns.values_mut() {
                col.drop_last_component(last_element_index);
            }
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
    /// - `row` must be in-bounds
    pub(crate) unsafe fn move_to_and_forget_missing_unchecked(
        &mut self,
        row: TableRow,
        new_table: &mut Table,
    ) -> TableMoveResult {
        debug_assert!(row.as_usize() < self.entity_count());
        let last_element_index = self.entity_count() - 1;
        let is_last = row.as_usize() == last_element_index;
        let new_row = new_table.allocate(self.entities.swap_remove(row.as_usize()));
        for (component_id, column) in self.columns.iter_mut() {
            if let Some(new_column) = new_table.get_column_mut(*component_id) {
                new_column.initialize_from_unchecked(column, last_element_index, row, new_row);
            } else {
                // It's the caller's responsibility to drop these cases.
                column.swap_remove_and_forget_unchecked(last_element_index, row);
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
        let last_element_index = self.entity_count() - 1;
        let is_last = row.as_usize() == last_element_index;
        let new_row = new_table.allocate(self.entities.swap_remove(row.as_usize()));
        for (component_id, column) in self.columns.iter_mut() {
            if let Some(new_column) = new_table.get_column_mut(*component_id) {
                new_column.initialize_from_unchecked(column, last_element_index, row, new_row);
            } else {
                column.swap_remove_and_drop_unchecked(last_element_index, row);
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

    /// Writes component data to its corresponding column at the given row.
    /// Assumes the slot is uninitialized, drop is not called.
    /// To overwrite an existing initialized value, use [`Self::replace_component`] instead.
    ///
    /// # Safety
    /// - `row.as_usize()` < `len`
    /// - `comp_ptr` holds a component that matches the `component_id`
    pub(crate) unsafe fn initialize_component(
        &mut self,
        row: TableRow,
        component_id: ComponentId,
        comp_ptr: OwningPtr<'_>,
        change_tick: Tick,
        #[cfg(feature = "track_change_detection")] caller: &'static Location<'static>,
    ) {
        debug_assert!(row.as_usize() < self.entity_count());
        self.get_column_mut(component_id)
            .debug_checked_unwrap()
            .initialize(
                row,
                comp_ptr,
                change_tick,
                #[cfg(feature = "track_change_detection")]
                caller,
            );
    }

    /// Replace the component at the given `row` with the component at `comp_ptr`
    ///
    /// # Safety
    /// `row.as_usize()` < `self.len()`
    /// `comp_ptr` holds the data of a component matching the `component_id`
    pub(crate) unsafe fn replace_component(
        &mut self,
        row: TableRow,
        component_id: ComponentId,
        comp_ptr: OwningPtr<'_>,
        change_tick: Tick,
        #[cfg(feature = "track_change_detection")] caller: &'static Location<'static>,
    ) {
        debug_assert!(row.as_usize() < self.entity_count());
        self.get_column_mut(component_id)
            .debug_checked_unwrap()
            .replace(
                row,
                comp_ptr,
                change_tick,
                #[cfg(feature = "track_change_detection")]
                caller,
            );
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
        let last_element_index = self.entity_count() - 1;
        let is_last = row.as_usize() == last_element_index;
        let new_row = new_table.allocate(self.entities.swap_remove(row.as_usize()));
        for (component_id, column) in self.columns.iter_mut() {
            new_table
                .get_column_mut(*component_id)
                .debug_checked_unwrap()
                .initialize_from_unchecked(column, last_element_index, row, new_row);
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

    /// Get the data of the column matching `component_id` as a slice.
    ///
    /// # Safety
    /// `row.as_usize()` < `self.len()`
    /// - `T` must match the `component_id`
    pub unsafe fn get_data_slice_for<T>(
        &self,
        component_id: ComponentId,
    ) -> Option<&[UnsafeCell<T>]> {
        self.get_column(component_id)
            .map(|col| col.get_data_slice_for(self.entity_count()))
    }

    /// Get the added ticks of the column matching `component_id` as a slice.
    pub fn get_added_ticks_slice_for(
        &self,
        component_id: ComponentId,
    ) -> Option<&[UnsafeCell<Tick>]> {
        self.get_column(component_id)
            // SAFETY: `self.len()` is guaranteed to be the len of the ticks array
            .map(|col| unsafe { col.get_added_ticks_slice(self.entity_count()) })
    }

    /// Get the changed ticks of the column matching `component_id` as a slice.
    pub fn get_changed_ticks_slice_for(
        &self,
        component_id: ComponentId,
    ) -> Option<&[UnsafeCell<Tick>]> {
        self.get_column(component_id)
            // SAFETY: `self.len()` is guaranteed to be the len of the ticks array
            .map(|col| unsafe { col.get_changed_ticks_slice(self.entity_count()) })
    }

    /// Fetches the calling locations that last changed the each component
    #[cfg(feature = "track_change_detection")]
    pub fn get_changed_by_slice_for(
        &self,
        component_id: ComponentId,
    ) -> Option<&[UnsafeCell<&'static Location<'static>>]> {
        self.get_column(component_id)
            // SAFETY: `self.len()` is guaranteed to be the len of the locations array
            .map(|col| unsafe { col.get_changed_by_slice(self.entity_count()) })
    }

    /// Get the specific [`change tick`](Tick) of the component matching `component_id` in `row`.
    pub fn get_changed_tick(
        &self,
        component_id: ComponentId,
        row: TableRow,
    ) -> Option<&UnsafeCell<Tick>> {
        (row.as_usize() < self.entity_count()).then_some(
            // SAFETY: `row.as_usize()` < `len`
            unsafe {
                self.get_column(component_id)?
                    .changed_ticks
                    .get_unchecked(row.as_usize())
            },
        )
    }

    /// Get the specific [`added tick`](Tick) of the component matching `component_id` in `row`.
    pub fn get_added_tick(
        &self,
        component_id: ComponentId,
        row: TableRow,
    ) -> Option<&UnsafeCell<Tick>> {
        (row.as_usize() < self.entity_count()).then_some(
            // SAFETY: `row.as_usize()` < `len`
            unsafe {
                self.get_column(component_id)?
                    .added_ticks
                    .get_unchecked(row.as_usize())
            },
        )
    }

    /// Get the specific calling location that changed the component matching `component_id` in `row`
    #[cfg(feature = "track_change_detection")]
    pub fn get_changed_by(
        &self,
        component_id: ComponentId,
        row: TableRow,
    ) -> Option<&UnsafeCell<&'static Location<'static>>> {
        (row.as_usize() < self.entity_count()).then_some(
            // SAFETY: `row.as_usize()` < `len`
            unsafe {
                self.get_column(component_id)?
                    .changed_by
                    .get_unchecked(row.as_usize())
            },
        )
    }

    /// Get the [`ComponentTicks`] of the component matching `component_id` in `row`.
    ///
    /// # Safety
    /// - `row.as_usize()` < `self.len()`
    pub unsafe fn get_ticks_unchecked(
        &self,
        component_id: ComponentId,
        row: TableRow,
    ) -> Option<ComponentTicks> {
        self.get_column(component_id).map(|col| ComponentTicks {
            added: col.added_ticks.get_unchecked(row.as_usize()).read(),
            changed: col.changed_ticks.get_unchecked(row.as_usize()).read(),
        })
    }

    /// Fetches a read-only reference to the [`ThinColumn`] for a given [`Component`] within the table.
    ///
    /// Returns `None` if the corresponding component does not belong to the table.
    ///
    /// [`Component`]: crate::component::Component
    #[inline]
    pub fn get_column(&self, component_id: ComponentId) -> Option<&ThinColumn> {
        self.columns.get(component_id)
    }

    /// Fetches a mutable reference to the [`ThinColumn`] for a given [`Component`] within the
    /// table.
    ///
    /// Returns `None` if the corresponding component does not belong to the table.
    ///
    /// [`Component`]: crate::component::Component
    #[inline]
    pub(crate) fn get_column_mut(&mut self, component_id: ComponentId) -> Option<&mut ThinColumn> {
        self.columns.get_mut(component_id)
    }

    /// Checks if the table contains a [`ThinColumn`] for a given [`Component`].
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
        if self.capacity() - self.entity_count() < additional {
            let column_cap = self.capacity();
            self.entities.reserve(additional);

            // use entities vector capacity as driving capacity for all related allocations
            let new_capacity = self.entities.capacity();

            if column_cap == 0 {
                // SAFETY: 0 < `column_cap` <= `new_capacity`
                unsafe { self.alloc_columns(NonZeroUsize::new_unchecked(new_capacity)) };
            } else {
                // SAFETY:
                // - `column_cap` is indeed the columns' capacity
                // - 0 < `additional` <= `self.len() + additional` <= `new_capacity`
                unsafe {
                    self.realloc_columns(
                        NonZeroUsize::new_unchecked(column_cap),
                        NonZeroUsize::new_unchecked(new_capacity),
                    );
                };
            }
        }
    }

    /// Allocate memory for the columns in the [`Table`]
    ///
    /// The current capacity of the columns should be 0, if it's not 0, then the previous data will be overwritten and leaked.
    fn alloc_columns(&mut self, new_capacity: NonZeroUsize) {
        // If any of these allocations trigger an unwind, the wrong capacity will be used while dropping this table - UB.
        // To avoid this, we use `AbortOnPanic`. If the allocation triggered a panic, the `AbortOnPanic`'s Drop impl will be
        // called, and abort the program.
        let _guard = AbortOnPanic;
        for col in self.columns.values_mut() {
            col.alloc(new_capacity);
        }
        core::mem::forget(_guard); // The allocation was successful, so we don't drop the guard.
    }

    /// Reallocate memory for the columns in the [`Table`]
    ///
    /// # Safety
    /// - `current_column_capacity` is indeed the capacity of the columns
    unsafe fn realloc_columns(
        &mut self,
        current_column_capacity: NonZeroUsize,
        new_capacity: NonZeroUsize,
    ) {
        // If any of these allocations trigger an unwind, the wrong capacity will be used while dropping this table - UB.
        // To avoid this, we use `AbortOnPanic`. If the allocation triggered a panic, the `AbortOnPanic`'s Drop impl will be
        // called, and abort the program.
        let _guard = AbortOnPanic;

        // SAFETY:
        // - There's no overflow
        // - `current_capacity` is indeed the capacity - safety requirement
        // - current capacity > 0
        for col in self.columns.values_mut() {
            col.realloc(current_column_capacity, new_capacity);
        }
        core::mem::forget(_guard); // The allocation was successful, so we don't drop the guard.
    }

    /// Allocates space for a new entity
    ///
    /// # Safety
    /// the allocated row must be written to immediately with valid values in each column
    pub(crate) unsafe fn allocate(&mut self, entity: Entity) -> TableRow {
        self.reserve(1);
        let len = self.entity_count();
        self.entities.push(entity);
        for col in self.columns.values_mut() {
            col.added_ticks
                .initialize_unchecked(len, UnsafeCell::new(Tick::new(0)));
            col.changed_ticks
                .initialize_unchecked(len, UnsafeCell::new(Tick::new(0)));
            #[cfg(feature = "track_change_detection")]
            col.changed_by
                .initialize_unchecked(len, UnsafeCell::new(Location::caller()));
        }
        TableRow::from_usize(len)
    }

    /// Gets the number of entities currently being stored in the table.
    #[inline]
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Get the drop function for some component that is stored in this table.
    #[inline]
    pub fn get_drop_for(&self, component_id: ComponentId) -> Option<unsafe fn(OwningPtr<'_>)> {
        self.get_column(component_id)?.data.drop
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

    /// Call [`Tick::check_tick`] on all of the ticks in the [`Table`]
    pub(crate) fn check_change_ticks(&mut self, change_tick: Tick) {
        let len = self.entity_count();
        for col in self.columns.values_mut() {
            // SAFETY: `len` is the actual length of the column
            unsafe { col.check_change_ticks(len, change_tick) };
        }
    }

    /// Iterates over the [`ThinColumn`]s of the [`Table`].
    pub fn iter_columns(&self) -> impl Iterator<Item = &ThinColumn> {
        self.columns.values()
    }

    /// Clears all of the stored components in the [`Table`].
    pub(crate) fn clear(&mut self) {
        let len = self.entity_count();
        for column in self.columns.values_mut() {
            // SAFETY: we defer `self.entities.clear()` until after clearing the columns,
            // so `self.len()` should match the columns' len
            unsafe { column.clear(len) };
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
    pub(crate) unsafe fn take_component(
        &mut self,
        component_id: ComponentId,
        row: TableRow,
    ) -> OwningPtr<'_> {
        self.get_column_mut(component_id)
            .debug_checked_unwrap()
            .data
            .get_unchecked_mut(row.as_usize())
            .promote()
    }

    /// Get the component at a given `row`, if the [`Table`] stores components with the given `component_id`
    ///
    /// # Safety
    /// `row.as_usize()` < `self.len()`
    pub unsafe fn get_component(
        &self,
        component_id: ComponentId,
        row: TableRow,
    ) -> Option<Ptr<'_>> {
        self.get_column(component_id)
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
        let len = self.entity_count();
        let cap = self.capacity();
        self.entities.clear();
        for col in self.columns.values_mut() {
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
    #[cfg(feature = "track_change_detection")]
    use std::panic::Location;

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
                        #[cfg(feature = "track_change_detection")]
                        Location::caller(),
                    );
                });
            };
        }

        assert_eq!(table.entity_capacity(), 256);
        assert_eq!(table.entity_count(), 200);
    }
}

//! Types for defining [`Archetype`]s, collections of entities that have the same set of
//! components.
//!
//! An archetype uniquely describes a group of entities that share the same components:
//! a world only has one archetype for each unique combination of components, and all
//! entities that have those components and only those components belong to that
//! archetype.
//!
//! Archetypes are not to be confused with [`Table`]s. Each archetype stores its table
//! components in one table, and each archetype uniquely points to one table, but multiple
//! archetypes may store their table components in the same table. These archetypes
//! differ only by the [`SparseSet`] components.
//!
//! Like tables, archetypes can be created but are never cleaned up. Empty archetypes are
//! not removed, and persist until the world is dropped.
//!
//! Archetypes can be fetched from [`Archetypes`], which is accessible via [`World::archetypes`].
//!
//! [`Table`]: crate::storage::Table
//! [`World::archetypes`]: crate::world::World::archetypes

use crate::{
    bundle::BundleId,
    component::{ComponentId, Components, RequiredComponentConstructor, StorageType},
    entity::{Entity, EntityLocation},
    event::Event,
    observer::Observers,
    storage::{ImmutableSparseSet, SparseArray, SparseSet, TableId, TableRow},
};
use alloc::{boxed::Box, vec::Vec};
use bevy_platform::collections::{hash_map::Entry, HashMap};
use core::{
    hash::Hash,
    ops::{Index, IndexMut, RangeFrom},
};
use nonmax::NonMaxU32;

#[derive(Event)]
#[expect(dead_code, reason = "Prepare for the upcoming Query as Entities")]
pub(crate) struct ArchetypeCreated(pub ArchetypeId);

/// An opaque location within a [`Archetype`].
///
/// This can be used in conjunction with [`ArchetypeId`] to find the exact location
/// of an [`Entity`] within a [`World`]. An entity's archetype and index can be
/// retrieved via [`Entities::get`].
///
/// [`World`]: crate::world::World
/// [`Entities::get`]: crate::entity::Entities
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
// SAFETY: Must be repr(transparent) due to the safety requirements on EntityLocation
#[repr(transparent)]
pub struct ArchetypeRow(NonMaxU32);

impl ArchetypeRow {
    /// Index indicating an invalid archetype row.
    /// This is meant to be used as a placeholder.
    // TODO: Deprecate in favor of options, since `INVALID` is, technically, valid.
    pub const INVALID: ArchetypeRow = ArchetypeRow(NonMaxU32::MAX);

    /// Creates a `ArchetypeRow`.
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

/// An opaque unique ID for a single [`Archetype`] within a [`World`].
///
/// Archetype IDs are only valid for a given World, and are not globally unique.
/// Attempting to use an archetype ID on a world that it wasn't sourced from will
/// not return the archetype with the same components. The only exception to this is
/// [`EMPTY`] which is guaranteed to be identical for all Worlds.
///
/// [`World`]: crate::world::World
/// [`EMPTY`]: ArchetypeId::EMPTY
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
// SAFETY: Must be repr(transparent) due to the safety requirements on EntityLocation
#[repr(transparent)]
pub struct ArchetypeId(u32);

impl ArchetypeId {
    /// The ID for the [`Archetype`] without any components.
    pub const EMPTY: ArchetypeId = ArchetypeId(0);
    /// # Safety:
    ///
    /// This must always have an all-1s bit pattern to ensure soundness in fast entity id space allocation.
    pub const INVALID: ArchetypeId = ArchetypeId(u32::MAX);

    /// Create an `ArchetypeId` from a plain value.
    ///
    /// This is useful if you need to store the `ArchetypeId` as a plain value,
    /// for example in a specialized data structure such as a bitset.
    ///
    /// While it doesn't break any safety invariants, you should ensure the
    /// values comes from a pre-existing [`ArchetypeId::index`] in this world
    /// to avoid panics and other unexpected behaviors.
    #[inline]
    pub const fn new(index: usize) -> Self {
        ArchetypeId(index as u32)
    }

    /// The plain value of this `ArchetypeId`.
    ///
    /// In bevy, this is mostly used to store archetype ids in [`FixedBitSet`]s.
    ///
    /// [`FixedBitSet`]: fixedbitset::FixedBitSet
    #[inline]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Used in [`ArchetypeAfterBundleInsert`] to track whether components in the bundle are newly
/// added or already existed in the entity's archetype.
#[derive(Copy, Clone, Eq, PartialEq)]
pub(crate) enum ComponentStatus {
    Added,
    Existing,
}

/// Used in [`Edges`] to cache the result of inserting a bundle into the source archetype.
pub(crate) struct ArchetypeAfterBundleInsert {
    /// The target archetype after the bundle is inserted into the source archetype.
    pub archetype_id: ArchetypeId,
    /// For each component iterated in the same order as the source [`Bundle`](crate::bundle::Bundle),
    /// indicate if the component is newly added to the target archetype or if it already existed.
    pub bundle_status: Vec<ComponentStatus>,
    /// The set of additional required components that must be initialized immediately when adding this Bundle.
    ///
    /// The initial values are determined based on the provided constructor, falling back to the `Default` trait if none is given.
    pub required_components: Vec<RequiredComponentConstructor>,
    /// The components added by this bundle. This includes any Required Components that are inserted when adding this bundle.
    pub added: Vec<ComponentId>,
    /// The components that were explicitly contributed by this bundle, but already existed in the archetype. This _does not_ include any
    /// Required Components.
    pub existing: Vec<ComponentId>,
}

impl ArchetypeAfterBundleInsert {
    pub(crate) fn iter_inserted(&self) -> impl Iterator<Item = ComponentId> + Clone + '_ {
        self.added.iter().chain(self.existing.iter()).copied()
    }

    pub(crate) fn iter_added(&self) -> impl Iterator<Item = ComponentId> + Clone + '_ {
        self.added.iter().copied()
    }

    pub(crate) fn iter_existing(&self) -> impl Iterator<Item = ComponentId> + Clone + '_ {
        self.existing.iter().copied()
    }
}

/// This trait is used to report the status of [`Bundle`](crate::bundle::Bundle) components
/// being inserted into a given entity, relative to that entity's original archetype.
/// See [`crate::bundle::BundleInfo::write_components`] for more info.
pub(crate) trait BundleComponentStatus {
    /// Returns the Bundle's component status for the given "bundle index".
    ///
    /// # Safety
    /// Callers must ensure that index is always a valid bundle index for the
    /// Bundle associated with this [`BundleComponentStatus`]
    unsafe fn get_status(&self, index: usize) -> ComponentStatus;
}

impl BundleComponentStatus for ArchetypeAfterBundleInsert {
    #[inline]
    unsafe fn get_status(&self, index: usize) -> ComponentStatus {
        // SAFETY: caller has ensured index is a valid bundle index for this bundle
        unsafe { *self.bundle_status.get_unchecked(index) }
    }
}

pub(crate) struct SpawnBundleStatus;

impl BundleComponentStatus for SpawnBundleStatus {
    #[inline]
    unsafe fn get_status(&self, _index: usize) -> ComponentStatus {
        // Components inserted during a spawn call are always treated as added.
        ComponentStatus::Added
    }
}

/// Archetypes and bundles form a graph. Adding or removing a bundle moves
/// an [`Entity`] to a new [`Archetype`].
///
/// [`Edges`] caches the results of these moves. Each archetype caches
/// the result of a structural alteration. This can be used to monitor the
/// state of the archetype graph.
///
/// Note: This type only contains edges the [`World`] has already traversed.
/// If any of functions return `None`, it doesn't mean there is guaranteed
/// not to be a result of adding or removing that bundle, but rather that
/// operation that has moved an entity along that edge has not been performed
/// yet.
///
/// [`World`]: crate::world::World
#[derive(Default)]
pub struct Edges {
    insert_bundle: SparseArray<BundleId, ArchetypeAfterBundleInsert>,
    remove_bundle: SparseArray<BundleId, Option<ArchetypeId>>,
    take_bundle: SparseArray<BundleId, Option<ArchetypeId>>,
}

impl Edges {
    /// Checks the cache for the target archetype when inserting a bundle into the
    /// source archetype.
    ///
    /// If this returns `None`, it means there has not been a transition from
    /// the source archetype via the provided bundle.
    #[inline]
    pub fn get_archetype_after_bundle_insert(&self, bundle_id: BundleId) -> Option<ArchetypeId> {
        self.get_archetype_after_bundle_insert_internal(bundle_id)
            .map(|bundle| bundle.archetype_id)
    }

    /// Internal version of `get_archetype_after_bundle_insert` that
    /// fetches the full `ArchetypeAfterBundleInsert`.
    #[inline]
    pub(crate) fn get_archetype_after_bundle_insert_internal(
        &self,
        bundle_id: BundleId,
    ) -> Option<&ArchetypeAfterBundleInsert> {
        self.insert_bundle.get(bundle_id)
    }

    /// Caches the target archetype when inserting a bundle into the source archetype.
    #[inline]
    pub(crate) fn cache_archetype_after_bundle_insert(
        &mut self,
        bundle_id: BundleId,
        archetype_id: ArchetypeId,
        bundle_status: Vec<ComponentStatus>,
        required_components: Vec<RequiredComponentConstructor>,
        added: Vec<ComponentId>,
        existing: Vec<ComponentId>,
    ) {
        self.insert_bundle.insert(
            bundle_id,
            ArchetypeAfterBundleInsert {
                archetype_id,
                bundle_status,
                required_components,
                added,
                existing,
            },
        );
    }

    /// Checks the cache for the target archetype when removing a bundle from the
    /// source archetype.
    ///
    /// If this returns `None`, it means there has not been a transition from
    /// the source archetype via the provided bundle.
    ///
    /// If this returns `Some(None)`, it means that the bundle cannot be removed
    /// from the source archetype.
    #[inline]
    pub fn get_archetype_after_bundle_remove(
        &self,
        bundle_id: BundleId,
    ) -> Option<Option<ArchetypeId>> {
        self.remove_bundle.get(bundle_id).cloned()
    }

    /// Caches the target archetype when removing a bundle from the source archetype.
    #[inline]
    pub(crate) fn cache_archetype_after_bundle_remove(
        &mut self,
        bundle_id: BundleId,
        archetype_id: Option<ArchetypeId>,
    ) {
        self.remove_bundle.insert(bundle_id, archetype_id);
    }

    /// Checks the cache for the target archetype when taking a bundle from the
    /// source archetype.
    ///
    /// Unlike `remove`, `take` will only succeed if the source archetype
    /// contains all of the components in the bundle.
    ///
    /// If this returns `None`, it means there has not been a transition from
    /// the source archetype via the provided bundle.
    ///
    /// If this returns `Some(None)`, it means that the bundle cannot be taken
    /// from the source archetype.
    #[inline]
    pub fn get_archetype_after_bundle_take(
        &self,
        bundle_id: BundleId,
    ) -> Option<Option<ArchetypeId>> {
        self.take_bundle.get(bundle_id).cloned()
    }

    /// Caches the target archetype when taking a bundle from the source archetype.
    ///
    /// Unlike `remove`, `take` will only succeed if the source archetype
    /// contains all of the components in the bundle.
    #[inline]
    pub(crate) fn cache_archetype_after_bundle_take(
        &mut self,
        bundle_id: BundleId,
        archetype_id: Option<ArchetypeId>,
    ) {
        self.take_bundle.insert(bundle_id, archetype_id);
    }
}

/// Metadata about an [`Entity`] in a [`Archetype`].
pub struct ArchetypeEntity {
    entity: Entity,
    table_row: TableRow,
}

impl ArchetypeEntity {
    /// The ID of the entity.
    #[inline]
    pub const fn id(&self) -> Entity {
        self.entity
    }

    /// The row in the [`Table`] where the entity's components are stored.
    ///
    /// [`Table`]: crate::storage::Table
    #[inline]
    pub const fn table_row(&self) -> TableRow {
        self.table_row
    }
}

/// Internal metadata for an [`Entity`] getting removed from an [`Archetype`].
pub(crate) struct ArchetypeSwapRemoveResult {
    /// If the [`Entity`] was not the last in the [`Archetype`], it gets removed by swapping it out
    /// with the last entity in the archetype. In that case, this field contains the swapped entity.
    pub(crate) swapped_entity: Option<Entity>,
    /// The [`TableRow`] where the removed entity's components are stored.
    pub(crate) table_row: TableRow,
}

/// Internal metadata for a [`Component`] within a given [`Archetype`].
///
/// [`Component`]: crate::component::Component
struct ArchetypeComponentInfo {
    storage_type: StorageType,
}

bitflags::bitflags! {
    /// Flags used to keep track of metadata about the component in this [`Archetype`]
    ///
    /// Used primarily to early-out when there are no [`ComponentHook`] registered for any contained components.
    #[derive(Clone, Copy)]
    pub(crate) struct ArchetypeFlags: u32 {
        const ON_ADD_HOOK    = (1 << 0);
        const ON_INSERT_HOOK = (1 << 1);
        const ON_REPLACE_HOOK = (1 << 2);
        const ON_REMOVE_HOOK = (1 << 3);
        const ON_DESPAWN_HOOK = (1 << 4);
        const ON_ADD_OBSERVER = (1 << 5);
        const ON_INSERT_OBSERVER = (1 << 6);
        const ON_REPLACE_OBSERVER = (1 << 7);
        const ON_REMOVE_OBSERVER = (1 << 8);
        const ON_DESPAWN_OBSERVER = (1 << 9);
    }
}

/// Metadata for a single archetype within a [`World`].
///
/// For more information, see the *[module level documentation]*.
///
/// [`World`]: crate::world::World
/// [module level documentation]: crate::archetype
pub struct Archetype {
    id: ArchetypeId,
    table_id: TableId,
    edges: Edges,
    entities: Vec<ArchetypeEntity>,
    components: ImmutableSparseSet<ComponentId, ArchetypeComponentInfo>,
    pub(crate) flags: ArchetypeFlags,
}

impl Archetype {
    /// `table_components` and `sparse_set_components` must be sorted
    pub(crate) fn new(
        components: &Components,
        component_index: &mut ComponentIndex,
        observers: &Observers,
        id: ArchetypeId,
        table_id: TableId,
        table_components: impl Iterator<Item = ComponentId>,
        sparse_set_components: impl Iterator<Item = ComponentId>,
    ) -> Self {
        let (min_table, _) = table_components.size_hint();
        let (min_sparse, _) = sparse_set_components.size_hint();
        let mut flags = ArchetypeFlags::empty();
        let mut archetype_components = SparseSet::with_capacity(min_table + min_sparse);
        for (idx, component_id) in table_components.enumerate() {
            // SAFETY: We are creating an archetype that includes this component so it must exist
            let info = unsafe { components.get_info_unchecked(component_id) };
            info.update_archetype_flags(&mut flags);
            observers.update_archetype_flags(component_id, &mut flags);
            archetype_components.insert(
                component_id,
                ArchetypeComponentInfo {
                    storage_type: StorageType::Table,
                },
            );
            // NOTE: the `table_components` are sorted AND they were inserted in the `Table` in the same
            // sorted order, so the index of the `Column` in the `Table` is the same as the index of the
            // component in the `table_components` vector
            component_index
                .entry(component_id)
                .or_default()
                .insert(id, ArchetypeRecord { column: Some(idx) });
        }

        for component_id in sparse_set_components {
            // SAFETY: We are creating an archetype that includes this component so it must exist
            let info = unsafe { components.get_info_unchecked(component_id) };
            info.update_archetype_flags(&mut flags);
            observers.update_archetype_flags(component_id, &mut flags);
            archetype_components.insert(
                component_id,
                ArchetypeComponentInfo {
                    storage_type: StorageType::SparseSet,
                },
            );
            component_index
                .entry(component_id)
                .or_default()
                .insert(id, ArchetypeRecord { column: None });
        }
        Self {
            id,
            table_id,
            entities: Vec::new(),
            components: archetype_components.into_immutable(),
            edges: Default::default(),
            flags,
        }
    }

    /// Fetches the ID for the archetype.
    #[inline]
    pub fn id(&self) -> ArchetypeId {
        self.id
    }

    /// Fetches the flags for the archetype.
    #[inline]
    pub(crate) fn flags(&self) -> ArchetypeFlags {
        self.flags
    }

    /// Fetches the archetype's [`Table`] ID.
    ///
    /// [`Table`]: crate::storage::Table
    #[inline]
    pub fn table_id(&self) -> TableId {
        self.table_id
    }

    /// Fetches the entities contained in this archetype.
    #[inline]
    pub fn entities(&self) -> &[ArchetypeEntity] {
        &self.entities
    }

    /// Fetches the entities contained in this archetype.
    #[inline]
    pub fn entities_with_location(&self) -> impl Iterator<Item = (Entity, EntityLocation)> {
        self.entities.iter().enumerate().map(
            |(archetype_row, &ArchetypeEntity { entity, table_row })| {
                (
                    entity,
                    EntityLocation {
                        archetype_id: self.id,
                        // SAFETY: The entities in the archetype must be unique and there are never more than u32::MAX entities.
                        archetype_row: unsafe {
                            ArchetypeRow::new(NonMaxU32::new_unchecked(archetype_row as u32))
                        },
                        table_id: self.table_id,
                        table_row,
                    },
                )
            },
        )
    }

    /// Gets an iterator of all of the components stored in [`Table`]s.
    ///
    /// All of the IDs are unique.
    ///
    /// [`Table`]: crate::storage::Table
    #[inline]
    pub fn table_components(&self) -> impl Iterator<Item = ComponentId> + '_ {
        self.components
            .iter()
            .filter(|(_, component)| component.storage_type == StorageType::Table)
            .map(|(id, _)| *id)
    }

    /// Gets an iterator of all of the components stored in [`ComponentSparseSet`]s.
    ///
    /// All of the IDs are unique.
    ///
    /// [`ComponentSparseSet`]: crate::storage::ComponentSparseSet
    #[inline]
    pub fn sparse_set_components(&self) -> impl Iterator<Item = ComponentId> + '_ {
        self.components
            .iter()
            .filter(|(_, component)| component.storage_type == StorageType::SparseSet)
            .map(|(id, _)| *id)
    }

    /// Gets an iterator of all of the components in the archetype.
    ///
    /// All of the IDs are unique.
    #[inline]
    pub fn components(&self) -> impl Iterator<Item = ComponentId> + Clone + '_ {
        self.components.indices()
    }

    /// Returns the total number of components in the archetype
    #[inline]
    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// Fetches an immutable reference to the archetype's [`Edges`], a cache of
    /// archetypal relationships.
    #[inline]
    pub fn edges(&self) -> &Edges {
        &self.edges
    }

    /// Fetches a mutable reference to the archetype's [`Edges`], a cache of
    /// archetypal relationships.
    #[inline]
    pub(crate) fn edges_mut(&mut self) -> &mut Edges {
        &mut self.edges
    }

    /// Fetches the row in the [`Table`] where the components for the entity at `index`
    /// is stored.
    ///
    /// An entity's archetype row can be fetched from [`EntityLocation::archetype_row`], which
    /// can be retrieved from [`Entities::get`].
    ///
    /// # Panics
    /// This function will panic if `index >= self.len()`.
    ///
    /// [`Table`]: crate::storage::Table
    /// [`EntityLocation::archetype_row`]: crate::entity::EntityLocation::archetype_row
    /// [`Entities::get`]: crate::entity::Entities::get
    #[inline]
    pub fn entity_table_row(&self, row: ArchetypeRow) -> TableRow {
        self.entities[row.index()].table_row
    }

    /// Updates if the components for the entity at `index` can be found
    /// in the corresponding table.
    ///
    /// # Panics
    /// This function will panic if `index >= self.len()`.
    #[inline]
    pub(crate) fn set_entity_table_row(&mut self, row: ArchetypeRow, table_row: TableRow) {
        self.entities[row.index()].table_row = table_row;
    }

    /// Allocates an entity to the archetype.
    ///
    /// # Safety
    /// valid component values must be immediately written to the relevant storages
    /// `table_row` must be valid
    #[inline]
    pub(crate) unsafe fn allocate(
        &mut self,
        entity: Entity,
        table_row: TableRow,
    ) -> EntityLocation {
        // SAFETY: An entity can not have multiple archetype rows and there can not be more than u32::MAX entities.
        let archetype_row = unsafe { ArchetypeRow::new(NonMaxU32::new_unchecked(self.len())) };
        self.entities.push(ArchetypeEntity { entity, table_row });

        EntityLocation {
            archetype_id: self.id,
            archetype_row,
            table_id: self.table_id,
            table_row,
        }
    }

    #[inline]
    pub(crate) fn reserve(&mut self, additional: usize) {
        self.entities.reserve(additional);
    }

    /// Removes the entity at `row` by swapping it out. Returns the table row the entity is stored
    /// in.
    ///
    /// # Panics
    /// This function will panic if `row >= self.entities.len()`
    #[inline]
    pub(crate) fn swap_remove(&mut self, row: ArchetypeRow) -> ArchetypeSwapRemoveResult {
        let is_last = row.index() == self.entities.len() - 1;
        let entity = self.entities.swap_remove(row.index());
        ArchetypeSwapRemoveResult {
            swapped_entity: if is_last {
                None
            } else {
                Some(self.entities[row.index()].entity)
            },
            table_row: entity.table_row,
        }
    }

    /// Gets the total number of entities that belong to the archetype.
    #[inline]
    pub fn len(&self) -> u32 {
        // No entity may have more than one archetype row, so there are no duplicates,
        // and there may only ever be u32::MAX entities, so the length never exceeds u32's capacity.
        self.entities.len() as u32
    }

    /// Checks if the archetype has any entities.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Checks if the archetype contains a specific component. This runs in `O(1)` time.
    #[inline]
    pub fn contains(&self, component_id: ComponentId) -> bool {
        self.components.contains(component_id)
    }

    /// Gets the type of storage where a component in the archetype can be found.
    /// Returns `None` if the component is not part of the archetype.
    /// This runs in `O(1)` time.
    #[inline]
    pub fn get_storage_type(&self, component_id: ComponentId) -> Option<StorageType> {
        self.components
            .get(component_id)
            .map(|info| info.storage_type)
    }

    /// Clears all entities from the archetype.
    pub(crate) fn clear_entities(&mut self) {
        self.entities.clear();
    }

    /// Returns true if any of the components in this archetype have `on_add` hooks
    #[inline]
    pub fn has_add_hook(&self) -> bool {
        self.flags().contains(ArchetypeFlags::ON_ADD_HOOK)
    }

    /// Returns true if any of the components in this archetype have `on_insert` hooks
    #[inline]
    pub fn has_insert_hook(&self) -> bool {
        self.flags().contains(ArchetypeFlags::ON_INSERT_HOOK)
    }

    /// Returns true if any of the components in this archetype have `on_replace` hooks
    #[inline]
    pub fn has_replace_hook(&self) -> bool {
        self.flags().contains(ArchetypeFlags::ON_REPLACE_HOOK)
    }

    /// Returns true if any of the components in this archetype have `on_remove` hooks
    #[inline]
    pub fn has_remove_hook(&self) -> bool {
        self.flags().contains(ArchetypeFlags::ON_REMOVE_HOOK)
    }

    /// Returns true if any of the components in this archetype have `on_despawn` hooks
    #[inline]
    pub fn has_despawn_hook(&self) -> bool {
        self.flags().contains(ArchetypeFlags::ON_DESPAWN_HOOK)
    }

    /// Returns true if any of the components in this archetype have at least one [`Add`] observer
    ///
    /// [`Add`]: crate::lifecycle::Add
    #[inline]
    pub fn has_add_observer(&self) -> bool {
        self.flags().contains(ArchetypeFlags::ON_ADD_OBSERVER)
    }

    /// Returns true if any of the components in this archetype have at least one [`Insert`] observer
    ///
    /// [`Insert`]: crate::lifecycle::Insert
    #[inline]
    pub fn has_insert_observer(&self) -> bool {
        self.flags().contains(ArchetypeFlags::ON_INSERT_OBSERVER)
    }

    /// Returns true if any of the components in this archetype have at least one [`Replace`] observer
    ///
    /// [`Replace`]: crate::lifecycle::Replace
    #[inline]
    pub fn has_replace_observer(&self) -> bool {
        self.flags().contains(ArchetypeFlags::ON_REPLACE_OBSERVER)
    }

    /// Returns true if any of the components in this archetype have at least one [`Remove`] observer
    ///
    /// [`Remove`]: crate::lifecycle::Remove
    #[inline]
    pub fn has_remove_observer(&self) -> bool {
        self.flags().contains(ArchetypeFlags::ON_REMOVE_OBSERVER)
    }

    /// Returns true if any of the components in this archetype have at least one [`Despawn`] observer
    ///
    /// [`Despawn`]: crate::lifecycle::Despawn
    #[inline]
    pub fn has_despawn_observer(&self) -> bool {
        self.flags().contains(ArchetypeFlags::ON_DESPAWN_OBSERVER)
    }
}

/// The next [`ArchetypeId`] in an [`Archetypes`] collection.
///
/// This is used in archetype update methods to limit archetype updates to the
/// ones added since the last time the method ran.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ArchetypeGeneration(pub(crate) ArchetypeId);

impl ArchetypeGeneration {
    /// The first archetype.
    #[inline]
    pub const fn initial() -> Self {
        ArchetypeGeneration(ArchetypeId::EMPTY)
    }
}

#[derive(Hash, PartialEq, Eq)]
struct ArchetypeComponents {
    table_components: Box<[ComponentId]>,
    sparse_set_components: Box<[ComponentId]>,
}

/// Maps a [`ComponentId`] to the list of [`Archetypes`]([`Archetype`]) that contain the [`Component`](crate::component::Component),
/// along with an [`ArchetypeRecord`] which contains some metadata about how the component is stored in the archetype.
pub type ComponentIndex = HashMap<ComponentId, HashMap<ArchetypeId, ArchetypeRecord>>;

/// The backing store of all [`Archetype`]s within a [`World`].
///
/// For more information, see the *[module level documentation]*.
///
/// [`World`]: crate::world::World
/// [module level documentation]: crate::archetype
pub struct Archetypes {
    pub(crate) archetypes: Vec<Archetype>,
    /// find the archetype id by the archetype's components
    by_components: HashMap<ArchetypeComponents, ArchetypeId>,
    /// find all the archetypes that contain a component
    pub(crate) by_component: ComponentIndex,
}

/// Metadata about how a component is stored in an [`Archetype`].
pub struct ArchetypeRecord {
    /// Index of the component in the archetype's [`Table`](crate::storage::Table),
    /// or None if the component is a sparse set component.
    #[expect(
        dead_code,
        reason = "Currently unused, but planned to be used to implement a component index to improve performance of fragmenting relations."
    )]
    pub(crate) column: Option<usize>,
}

impl Archetypes {
    pub(crate) fn new() -> Self {
        let mut archetypes = Archetypes {
            archetypes: Vec::new(),
            by_components: Default::default(),
            by_component: Default::default(),
        };
        // SAFETY: Empty archetype has no components
        unsafe {
            archetypes.get_id_or_insert(
                &Components::default(),
                &Observers::default(),
                TableId::empty(),
                Vec::new(),
                Vec::new(),
            );
        }
        archetypes
    }

    /// Returns the "generation", a handle to the current highest archetype ID.
    ///
    /// This can be used with the `Index` [`Archetypes`] implementation to
    /// iterate over newly introduced [`Archetype`]s since the last time this
    /// function was called.
    #[inline]
    pub fn generation(&self) -> ArchetypeGeneration {
        let id = ArchetypeId::new(self.archetypes.len());
        ArchetypeGeneration(id)
    }

    /// Fetches the total number of [`Archetype`]s within the world.
    #[inline]
    #[expect(
        clippy::len_without_is_empty,
        reason = "The internal vec is never empty"
    )]
    pub fn len(&self) -> usize {
        self.archetypes.len()
    }

    /// Fetches an immutable reference to the archetype without any components.
    ///
    /// Shorthand for `archetypes.get(ArchetypeId::EMPTY).unwrap()`
    #[inline]
    pub fn empty(&self) -> &Archetype {
        // SAFETY: empty archetype always exists
        unsafe { self.archetypes.get_unchecked(ArchetypeId::EMPTY.index()) }
    }

    /// Fetches a mutable reference to the archetype without any components.
    #[inline]
    pub(crate) fn empty_mut(&mut self) -> &mut Archetype {
        // SAFETY: empty archetype always exists
        unsafe {
            self.archetypes
                .get_unchecked_mut(ArchetypeId::EMPTY.index())
        }
    }

    /// Fetches an immutable reference to an [`Archetype`] using its
    /// ID. Returns `None` if no corresponding archetype exists.
    #[inline]
    pub fn get(&self, id: ArchetypeId) -> Option<&Archetype> {
        self.archetypes.get(id.index())
    }

    /// # Panics
    ///
    /// Panics if `a` and `b` are equal.
    #[inline]
    pub(crate) fn get_2_mut(
        &mut self,
        a: ArchetypeId,
        b: ArchetypeId,
    ) -> (&mut Archetype, &mut Archetype) {
        if a.index() > b.index() {
            let (b_slice, a_slice) = self.archetypes.split_at_mut(a.index());
            (&mut a_slice[0], &mut b_slice[b.index()])
        } else {
            let (a_slice, b_slice) = self.archetypes.split_at_mut(b.index());
            (&mut a_slice[a.index()], &mut b_slice[0])
        }
    }

    /// Returns a read-only iterator over all archetypes.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &Archetype> {
        self.archetypes.iter()
    }

    /// Gets the archetype id matching the given inputs or inserts a new one if it doesn't exist.
    ///
    /// Specifically, it returns a tuple where the first element
    /// is the [`ArchetypeId`] that the given inputs belong to, and the second element is a boolean indicating whether a new archetype was created.
    ///
    /// `table_components` and `sparse_set_components` must be sorted
    ///
    /// # Safety
    /// [`TableId`] must exist in tables
    /// `table_components` and `sparse_set_components` must exist in `components`
    pub(crate) unsafe fn get_id_or_insert(
        &mut self,
        components: &Components,
        observers: &Observers,
        table_id: TableId,
        table_components: Vec<ComponentId>,
        sparse_set_components: Vec<ComponentId>,
    ) -> (ArchetypeId, bool) {
        let archetype_identity = ArchetypeComponents {
            sparse_set_components: sparse_set_components.into_boxed_slice(),
            table_components: table_components.into_boxed_slice(),
        };

        let archetypes = &mut self.archetypes;
        let component_index = &mut self.by_component;
        match self.by_components.entry(archetype_identity) {
            Entry::Occupied(occupied) => (*occupied.get(), false),
            Entry::Vacant(vacant) => {
                let ArchetypeComponents {
                    table_components,
                    sparse_set_components,
                } = vacant.key();
                let id = ArchetypeId::new(archetypes.len());
                archetypes.push(Archetype::new(
                    components,
                    component_index,
                    observers,
                    id,
                    table_id,
                    table_components.iter().copied(),
                    sparse_set_components.iter().copied(),
                ));
                vacant.insert(id);
                (id, true)
            }
        }
    }

    /// Clears all entities from all archetypes.
    pub(crate) fn clear_entities(&mut self) {
        for archetype in &mut self.archetypes {
            archetype.clear_entities();
        }
    }

    /// Get the component index
    pub(crate) fn component_index(&self) -> &ComponentIndex {
        &self.by_component
    }

    pub(crate) fn update_flags(
        &mut self,
        component_id: ComponentId,
        flags: ArchetypeFlags,
        set: bool,
    ) {
        if let Some(archetypes) = self.by_component.get(&component_id) {
            for archetype_id in archetypes.keys() {
                // SAFETY: the component index only contains valid archetype ids
                self.archetypes
                    .get_mut(archetype_id.index())
                    .unwrap()
                    .flags
                    .set(flags, set);
            }
        }
    }
}

impl Index<RangeFrom<ArchetypeGeneration>> for Archetypes {
    type Output = [Archetype];

    #[inline]
    fn index(&self, index: RangeFrom<ArchetypeGeneration>) -> &Self::Output {
        &self.archetypes[index.start.0.index()..]
    }
}

impl Index<ArchetypeId> for Archetypes {
    type Output = Archetype;

    #[inline]
    fn index(&self, index: ArchetypeId) -> &Self::Output {
        &self.archetypes[index.index()]
    }
}

impl IndexMut<ArchetypeId> for Archetypes {
    #[inline]
    fn index_mut(&mut self, index: ArchetypeId) -> &mut Self::Output {
        &mut self.archetypes[index.index()]
    }
}

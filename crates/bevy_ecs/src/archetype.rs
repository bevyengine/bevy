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
    component::{ComponentId, StorageType},
    entity::{Entity, EntityLocation},
    storage::{ImmutableSparseSet, SparseArray, SparseSet, SparseSetIndex, TableId, TableRow},
};
use std::{
    hash::Hash,
    ops::{Index, IndexMut, RangeFrom},
};

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
pub struct ArchetypeRow(u32);

impl ArchetypeRow {
    /// Index indicating an invalid archetype row.
    /// This is meant to be used as a placeholder.
    pub const INVALID: ArchetypeRow = ArchetypeRow(u32::MAX);

    /// Creates a `ArchetypeRow`.
    #[inline]
    pub const fn new(index: usize) -> Self {
        Self(index as u32)
    }

    /// Gets the index of the row.
    #[inline]
    pub const fn index(self) -> usize {
        self.0 as usize
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
/// [`EMPTY`]: crate::archetype::ArchetypeId::EMPTY
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

#[derive(Copy, Clone)]
pub(crate) enum ComponentStatus {
    Added,
    Mutated,
}

pub(crate) struct AddBundle {
    pub archetype_id: ArchetypeId,
    pub bundle_status: Vec<ComponentStatus>,
}

/// This trait is used to report the status of [`Bundle`](crate::bundle::Bundle) components
/// being added to a given entity, relative to that entity's original archetype.
/// See [`crate::bundle::BundleInfo::write_components`] for more info.
pub(crate) trait BundleComponentStatus {
    /// Returns the Bundle's component status for the given "bundle index"
    ///
    /// # Safety
    /// Callers must ensure that index is always a valid bundle index for the
    /// Bundle associated with this [`BundleComponentStatus`]
    unsafe fn get_status(&self, index: usize) -> ComponentStatus;
}

impl BundleComponentStatus for AddBundle {
    #[inline]
    unsafe fn get_status(&self, index: usize) -> ComponentStatus {
        // SAFETY: caller has ensured index is a valid bundle index for this bundle
        *self.bundle_status.get_unchecked(index)
    }
}

pub(crate) struct SpawnBundleStatus;

impl BundleComponentStatus for SpawnBundleStatus {
    #[inline]
    unsafe fn get_status(&self, _index: usize) -> ComponentStatus {
        // Components added during a spawn call are always treated as added
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
    add_bundle: SparseArray<BundleId, AddBundle>,
    remove_bundle: SparseArray<BundleId, Option<ArchetypeId>>,
    take_bundle: SparseArray<BundleId, Option<ArchetypeId>>,
}

impl Edges {
    /// Checks the cache for the target archetype when adding a bundle to the
    /// source archetype. For more information, see [`EntityWorldMut::insert`].
    ///
    /// If this returns `None`, it means there has not been a transition from
    /// the source archetype via the provided bundle.
    ///
    /// [`EntityWorldMut::insert`]: crate::world::EntityWorldMut::insert
    #[inline]
    pub fn get_add_bundle(&self, bundle_id: BundleId) -> Option<ArchetypeId> {
        self.get_add_bundle_internal(bundle_id)
            .map(|bundle| bundle.archetype_id)
    }

    /// Internal version of `get_add_bundle` that fetches the full `AddBundle`.
    #[inline]
    pub(crate) fn get_add_bundle_internal(&self, bundle_id: BundleId) -> Option<&AddBundle> {
        self.add_bundle.get(bundle_id)
    }

    /// Caches the target archetype when adding a bundle to the source archetype.
    /// For more information, see [`EntityWorldMut::insert`].
    ///
    /// [`EntityWorldMut::insert`]: crate::world::EntityWorldMut::insert
    #[inline]
    pub(crate) fn insert_add_bundle(
        &mut self,
        bundle_id: BundleId,
        archetype_id: ArchetypeId,
        bundle_status: Vec<ComponentStatus>,
    ) {
        self.add_bundle.insert(
            bundle_id,
            AddBundle {
                archetype_id,
                bundle_status,
            },
        );
    }

    /// Checks the cache for the target archetype when removing a bundle to the
    /// source archetype. For more information, see [`EntityWorldMut::remove`].
    ///
    /// If this returns `None`, it means there has not been a transition from
    /// the source archetype via the provided bundle.
    ///
    /// If this returns `Some(None)`, it means that the bundle cannot be removed
    /// from the source archetype.
    ///
    /// [`EntityWorldMut::remove`]: crate::world::EntityWorldMut::remove
    #[inline]
    pub fn get_remove_bundle(&self, bundle_id: BundleId) -> Option<Option<ArchetypeId>> {
        self.remove_bundle.get(bundle_id).cloned()
    }

    /// Caches the target archetype when removing a bundle to the source archetype.
    /// For more information, see [`EntityWorldMut::remove`].
    ///
    /// [`EntityWorldMut::remove`]: crate::world::EntityWorldMut::remove
    #[inline]
    pub(crate) fn insert_remove_bundle(
        &mut self,
        bundle_id: BundleId,
        archetype_id: Option<ArchetypeId>,
    ) {
        self.remove_bundle.insert(bundle_id, archetype_id);
    }

    /// Checks the cache for the target archetype when removing a bundle to the
    /// source archetype. For more information, see [`EntityWorldMut::remove`].
    ///
    /// If this returns `None`, it means there has not been a transition from
    /// the source archetype via the provided bundle.
    ///
    /// [`EntityWorldMut::remove`]: crate::world::EntityWorldMut::remove
    #[inline]
    pub fn get_take_bundle(&self, bundle_id: BundleId) -> Option<Option<ArchetypeId>> {
        self.take_bundle.get(bundle_id).cloned()
    }

    /// Caches the target archetype when removing a bundle to the source archetype.
    /// For more information, see [`EntityWorldMut::take`].
    ///
    /// [`EntityWorldMut::take`]: crate::world::EntityWorldMut::take
    #[inline]
    pub(crate) fn insert_take_bundle(
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
    pub const fn entity(&self) -> Entity {
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

pub(crate) struct ArchetypeSwapRemoveResult {
    pub(crate) swapped_entity: Option<Entity>,
    pub(crate) table_row: TableRow,
}

/// Internal metadata for a [`Component`] within a given [`Archetype`].
///
/// [`Component`]: crate::component::Component
struct ArchetypeComponentInfo {
    storage_type: StorageType,
    archetype_component_id: ArchetypeComponentId,
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
}

impl Archetype {
    pub(crate) fn new(
        id: ArchetypeId,
        table_id: TableId,
        table_components: impl Iterator<Item = (ComponentId, ArchetypeComponentId)>,
        sparse_set_components: impl Iterator<Item = (ComponentId, ArchetypeComponentId)>,
    ) -> Self {
        let (min_table, _) = table_components.size_hint();
        let (min_sparse, _) = sparse_set_components.size_hint();
        let mut components = SparseSet::with_capacity(min_table + min_sparse);
        for (component_id, archetype_component_id) in table_components {
            components.insert(
                component_id,
                ArchetypeComponentInfo {
                    storage_type: StorageType::Table,
                    archetype_component_id,
                },
            );
        }

        for (component_id, archetype_component_id) in sparse_set_components {
            components.insert(
                component_id,
                ArchetypeComponentInfo {
                    storage_type: StorageType::SparseSet,
                    archetype_component_id,
                },
            );
        }
        Self {
            id,
            table_id,
            entities: Vec::new(),
            components: components.into_immutable(),
            edges: Default::default(),
        }
    }

    /// Fetches the ID for the archetype.
    #[inline]
    pub fn id(&self) -> ArchetypeId {
        self.id
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
    pub fn components(&self) -> impl Iterator<Item = ComponentId> + '_ {
        self.components.indices()
    }

    /// Fetches a immutable reference to the archetype's [`Edges`], a cache of
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
        let archetype_row = ArchetypeRow::new(self.entities.len());
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

    /// Removes the entity at `index` by swapping it out. Returns the table row the entity is stored
    /// in.
    ///
    /// # Panics
    /// This function will panic if `index >= self.len()`
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
    pub fn len(&self) -> usize {
        self.entities.len()
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

    /// Fetches the corresponding [`ArchetypeComponentId`] for a component in the archetype.
    /// Returns `None` if the component is not part of the archetype.
    /// This runs in `O(1)` time.
    #[inline]
    pub fn get_archetype_component_id(
        &self,
        component_id: ComponentId,
    ) -> Option<ArchetypeComponentId> {
        self.components
            .get(component_id)
            .map(|info| info.archetype_component_id)
    }

    /// Clears all entities from the archetype.
    pub(crate) fn clear_entities(&mut self) {
        self.entities.clear();
    }
}

/// The next [`ArchetypeId`] in an [`Archetypes`] collection.
///
/// This is used in archetype update methods to limit archetype updates to the
/// ones added since the last time the method ran.
#[derive(Debug, Copy, Clone)]
pub struct ArchetypeGeneration(ArchetypeId);

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

/// An opaque unique joint ID for a [`Component`] in an [`Archetype`] within a [`World`].
///
/// A component may be present within multiple archetypes, but each component within
/// each archetype has its own unique `ArchetypeComponentId`. This is leveraged by the system
/// schedulers to opportunistically run multiple systems in parallel that would otherwise
/// conflict. For example, `Query<&mut A, With<B>>` and `Query<&mut A, Without<B>>` can run in
/// parallel as the matched `ArchetypeComponentId` sets for both queries are disjoint, even
/// though `&mut A` on both queries point to the same [`ComponentId`].
///
/// In SQL terms, these IDs are composite keys on a [many-to-many relationship] between archetypes
/// and components. Each component type will have only one [`ComponentId`], but may have many
/// [`ArchetypeComponentId`]s, one for every archetype the component is present in. Likewise, each
/// archetype will have only one [`ArchetypeId`] but may have many [`ArchetypeComponentId`]s, one
/// for each component that belongs to the archetype.
///
/// Every [`Resource`] is also assigned one of these IDs. As resources do not belong to any
/// particular archetype, a resource's ID uniquely identifies it.
///
/// These IDs are only valid within a given World, and are not globally unique.
/// Attempting to use an ID on a world that it wasn't sourced from will
/// not point to the same archetype nor the same component.
///
/// [`Component`]: crate::component::Component
/// [`World`]: crate::world::World
/// [`Resource`]: crate::system::Resource
/// [many-to-many relationship]: https://en.wikipedia.org/wiki/Many-to-many_(data_model)
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ArchetypeComponentId(usize);

impl ArchetypeComponentId {
    #[inline]
    pub(crate) const fn new(index: usize) -> Self {
        Self(index)
    }
}

impl SparseSetIndex for ArchetypeComponentId {
    #[inline]
    fn sparse_set_index(&self) -> usize {
        self.0
    }

    fn get_sparse_set_index(value: usize) -> Self {
        Self(value)
    }
}

/// The backing store of all [`Archetype`]s within a [`World`].
///
/// For more information, see the *[module level documentation]*.
///
/// [`World`]: crate::world::World
/// [module level documentation]: crate::archetype
pub struct Archetypes {
    pub(crate) archetypes: Vec<Archetype>,
    pub(crate) archetype_component_count: usize,
    by_components: bevy_utils::HashMap<ArchetypeComponents, ArchetypeId>,
}

impl Archetypes {
    pub(crate) fn new() -> Self {
        let mut archetypes = Archetypes {
            archetypes: Vec::new(),
            by_components: Default::default(),
            archetype_component_count: 0,
        };
        archetypes.get_id_or_insert(TableId::empty(), Vec::new(), Vec::new());
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
    #[allow(clippy::len_without_is_empty)] // the internal vec is never empty.
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

    /// Fetches an mutable reference to the archetype without any components.
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
    /// `table_components` and `sparse_set_components` must be sorted
    ///
    /// # Safety
    /// [`TableId`] must exist in tables
    pub(crate) fn get_id_or_insert(
        &mut self,
        table_id: TableId,
        table_components: Vec<ComponentId>,
        sparse_set_components: Vec<ComponentId>,
    ) -> ArchetypeId {
        let archetype_identity = ArchetypeComponents {
            sparse_set_components: sparse_set_components.clone().into_boxed_slice(),
            table_components: table_components.clone().into_boxed_slice(),
        };

        let archetypes = &mut self.archetypes;
        let archetype_component_count = &mut self.archetype_component_count;
        *self
            .by_components
            .entry(archetype_identity)
            .or_insert_with(move || {
                let id = ArchetypeId::new(archetypes.len());
                let table_start = *archetype_component_count;
                *archetype_component_count += table_components.len();
                let table_archetype_components =
                    (table_start..*archetype_component_count).map(ArchetypeComponentId);
                let sparse_start = *archetype_component_count;
                *archetype_component_count += sparse_set_components.len();
                let sparse_set_archetype_components =
                    (sparse_start..*archetype_component_count).map(ArchetypeComponentId);
                archetypes.push(Archetype::new(
                    id,
                    table_id,
                    table_components.into_iter().zip(table_archetype_components),
                    sparse_set_components
                        .into_iter()
                        .zip(sparse_set_archetype_components),
                ));
                id
            })
    }

    /// Returns the number of components that are stored in archetypes.
    /// Note that if some component `T` is stored in more than one archetype, it will be counted once for each archetype it's present in.
    #[inline]
    pub fn archetype_components_len(&self) -> usize {
        self.archetype_component_count
    }

    /// Clears all entities from all archetypes.
    pub(crate) fn clear_entities(&mut self) {
        for archetype in &mut self.archetypes {
            archetype.clear_entities();
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

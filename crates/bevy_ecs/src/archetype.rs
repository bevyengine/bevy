//! Types for defining [`Archetype`]s, collections of entities that have the same set of
//! components.

use crate::{
    bundle::BundleId,
    component::{ComponentId, StorageType},
    entity::{Entity, EntityLocation},
    storage::{Column, SparseArray, SparseSet, SparseSetIndex, TableId},
};
use std::{
    borrow::Cow,
    collections::HashMap,
    hash::Hash,
    ops::{Index, IndexMut},
};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ArchetypeId(usize);

impl ArchetypeId {
    pub const EMPTY: ArchetypeId = ArchetypeId(0);
    pub const RESOURCE: ArchetypeId = ArchetypeId(1);
    pub const INVALID: ArchetypeId = ArchetypeId(usize::MAX);

    #[inline]
    pub const fn new(index: usize) -> Self {
        ArchetypeId(index)
    }

    #[inline]
    pub fn index(self) -> usize {
        self.0
    }
}

pub enum ComponentStatus {
    Added,
    Mutated,
}

pub struct AddBundle {
    pub archetype_id: ArchetypeId,
    pub bundle_status: Vec<ComponentStatus>,
}

#[derive(Default)]
pub struct Edges {
    pub add_bundle: SparseArray<BundleId, AddBundle>,
    pub remove_bundle: SparseArray<BundleId, Option<ArchetypeId>>,
    pub remove_bundle_intersection: SparseArray<BundleId, Option<ArchetypeId>>,
}

impl Edges {
    #[inline]
    pub fn get_add_bundle(&self, bundle_id: BundleId) -> Option<&AddBundle> {
        self.add_bundle.get(bundle_id)
    }

    #[inline]
    pub fn insert_add_bundle(
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

    #[inline]
    pub fn get_remove_bundle(&self, bundle_id: BundleId) -> Option<Option<ArchetypeId>> {
        self.remove_bundle.get(bundle_id).cloned()
    }

    #[inline]
    pub fn insert_remove_bundle(&mut self, bundle_id: BundleId, archetype_id: Option<ArchetypeId>) {
        self.remove_bundle.insert(bundle_id, archetype_id);
    }

    #[inline]
    pub fn get_remove_bundle_intersection(
        &self,
        bundle_id: BundleId,
    ) -> Option<Option<ArchetypeId>> {
        self.remove_bundle_intersection.get(bundle_id).cloned()
    }

    #[inline]
    pub fn insert_remove_bundle_intersection(
        &mut self,
        bundle_id: BundleId,
        archetype_id: Option<ArchetypeId>,
    ) {
        self.remove_bundle_intersection
            .insert(bundle_id, archetype_id);
    }
}

struct TableInfo {
    id: TableId,
    entity_rows: Vec<usize>,
}

pub(crate) struct ArchetypeSwapRemoveResult {
    pub(crate) swapped_entity: Option<Entity>,
    pub(crate) table_row: usize,
}

pub(crate) struct ArchetypeComponentInfo {
    pub(crate) storage_type: StorageType,
    pub(crate) archetype_component_id: ArchetypeComponentId,
}

pub struct Archetype {
    id: ArchetypeId,
    entities: Vec<Entity>,
    edges: Edges,
    table_info: TableInfo,
    table_components: Cow<'static, [ComponentId]>,
    sparse_set_components: Cow<'static, [ComponentId]>,
    pub(crate) unique_components: SparseSet<ComponentId, Column>,
    pub(crate) components: SparseSet<ComponentId, ArchetypeComponentInfo>,
}

impl Archetype {
    pub fn new(
        id: ArchetypeId,
        table_id: TableId,
        table_components: Cow<'static, [ComponentId]>,
        sparse_set_components: Cow<'static, [ComponentId]>,
        table_archetype_components: Vec<ArchetypeComponentId>,
        sparse_set_archetype_components: Vec<ArchetypeComponentId>,
    ) -> Self {
        let mut components =
            SparseSet::with_capacity(table_components.len() + sparse_set_components.len());
        for (component_id, archetype_component_id) in
            table_components.iter().zip(table_archetype_components)
        {
            components.insert(
                *component_id,
                ArchetypeComponentInfo {
                    storage_type: StorageType::Table,
                    archetype_component_id,
                },
            );
        }

        for (component_id, archetype_component_id) in sparse_set_components
            .iter()
            .zip(sparse_set_archetype_components)
        {
            components.insert(
                *component_id,
                ArchetypeComponentInfo {
                    storage_type: StorageType::SparseSet,
                    archetype_component_id,
                },
            );
        }
        Self {
            id,
            table_info: TableInfo {
                id: table_id,
                entity_rows: Default::default(),
            },
            components,
            table_components,
            sparse_set_components,
            unique_components: SparseSet::new(),
            entities: Default::default(),
            edges: Default::default(),
        }
    }

    #[inline]
    pub fn id(&self) -> ArchetypeId {
        self.id
    }

    #[inline]
    pub fn table_id(&self) -> TableId {
        self.table_info.id
    }

    #[inline]
    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }

    #[inline]
    pub fn entity_table_rows(&self) -> &[usize] {
        &self.table_info.entity_rows
    }

    #[inline]
    pub fn table_components(&self) -> &[ComponentId] {
        &self.table_components
    }

    #[inline]
    pub fn sparse_set_components(&self) -> &[ComponentId] {
        &self.sparse_set_components
    }

    #[inline]
    pub fn unique_components(&self) -> &SparseSet<ComponentId, Column> {
        &self.unique_components
    }

    #[inline]
    pub fn unique_components_mut(&mut self) -> &mut SparseSet<ComponentId, Column> {
        &mut self.unique_components
    }

    #[inline]
    pub fn components(&self) -> impl Iterator<Item = ComponentId> + '_ {
        self.components.indices()
    }

    #[inline]
    pub fn edges(&self) -> &Edges {
        &self.edges
    }

    #[inline]
    pub(crate) fn edges_mut(&mut self) -> &mut Edges {
        &mut self.edges
    }

    #[inline]
    pub fn entity_table_row(&self, index: usize) -> usize {
        self.table_info.entity_rows[index]
    }

    #[inline]
    pub fn set_entity_table_row(&mut self, index: usize, table_row: usize) {
        self.table_info.entity_rows[index] = table_row;
    }

    /// # Safety
    /// valid component values must be immediately written to the relevant storages
    /// `table_row` must be valid
    pub unsafe fn allocate(&mut self, entity: Entity, table_row: usize) -> EntityLocation {
        self.entities.push(entity);
        self.table_info.entity_rows.push(table_row);

        EntityLocation {
            archetype_id: self.id,
            index: self.entities.len() - 1,
        }
    }

    pub fn reserve(&mut self, additional: usize) {
        self.entities.reserve(additional);
        self.table_info.entity_rows.reserve(additional);
    }

    /// Removes the entity at `index` by swapping it out. Returns the table row the entity is stored
    /// in.
    pub(crate) fn swap_remove(&mut self, index: usize) -> ArchetypeSwapRemoveResult {
        let is_last = index == self.entities.len() - 1;
        self.entities.swap_remove(index);
        ArchetypeSwapRemoveResult {
            swapped_entity: if is_last {
                None
            } else {
                Some(self.entities[index])
            },
            table_row: self.table_info.entity_rows.swap_remove(index),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    #[inline]
    pub fn contains(&self, component_id: ComponentId) -> bool {
        self.components.contains(component_id)
    }

    #[inline]
    pub fn get_storage_type(&self, component_id: ComponentId) -> Option<StorageType> {
        self.components
            .get(component_id)
            .map(|info| info.storage_type)
    }

    #[inline]
    pub fn get_archetype_component_id(
        &self,
        component_id: ComponentId,
    ) -> Option<ArchetypeComponentId> {
        self.components
            .get(component_id)
            .map(|info| info.archetype_component_id)
    }

    pub(crate) fn clear_entities(&mut self) {
        self.entities.clear();
        self.table_info.entity_rows.clear();
    }
}

/// A generational id that changes every time the set of archetypes changes
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ArchetypeGeneration(usize);

impl ArchetypeGeneration {
    #[inline]
    pub const fn initial() -> Self {
        ArchetypeGeneration(0)
    }

    #[inline]
    pub fn value(self) -> usize {
        self.0
    }
}

#[derive(Hash, PartialEq, Eq)]
pub struct ArchetypeIdentity {
    table_components: Cow<'static, [ComponentId]>,
    sparse_set_components: Cow<'static, [ComponentId]>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ArchetypeComponentId(usize);

impl ArchetypeComponentId {
    #[inline]
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    #[inline]
    pub fn index(self) -> usize {
        self.0
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

pub struct Archetypes {
    pub(crate) archetypes: Vec<Archetype>,
    pub(crate) archetype_component_count: usize,
    archetype_ids: HashMap<ArchetypeIdentity, ArchetypeId>,
}

impl Default for Archetypes {
    fn default() -> Self {
        let mut archetypes = Archetypes {
            archetypes: Vec::new(),
            archetype_ids: Default::default(),
            archetype_component_count: 0,
        };
        archetypes.get_id_or_insert(TableId::empty(), Vec::new(), Vec::new());

        // adds the resource archetype. it is "special" in that it is inaccessible via a "hash",
        // which prevents entities from being added to it
        archetypes.archetypes.push(Archetype::new(
            ArchetypeId::RESOURCE,
            TableId::empty(),
            Cow::Owned(Vec::new()),
            Cow::Owned(Vec::new()),
            Vec::new(),
            Vec::new(),
        ));
        archetypes
    }
}

impl Archetypes {
    #[inline]
    pub fn generation(&self) -> ArchetypeGeneration {
        ArchetypeGeneration(self.archetypes.len())
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.archetypes.len()
    }

    #[inline]
    pub fn empty(&self) -> &Archetype {
        // SAFE: empty archetype always exists
        unsafe { self.archetypes.get_unchecked(ArchetypeId::EMPTY.index()) }
    }

    #[inline]
    pub fn empty_mut(&mut self) -> &mut Archetype {
        // SAFE: empty archetype always exists
        unsafe {
            self.archetypes
                .get_unchecked_mut(ArchetypeId::EMPTY.index())
        }
    }

    #[inline]
    pub fn resource(&self) -> &Archetype {
        // SAFE: resource archetype always exists
        unsafe { self.archetypes.get_unchecked(ArchetypeId::RESOURCE.index()) }
    }

    #[inline]
    pub fn resource_mut(&mut self) -> &mut Archetype {
        // SAFE: resource archetype always exists
        unsafe {
            self.archetypes
                .get_unchecked_mut(ArchetypeId::RESOURCE.index())
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.archetypes.is_empty()
    }

    #[inline]
    pub fn get(&self, id: ArchetypeId) -> Option<&Archetype> {
        self.archetypes.get(id.index())
    }

    #[inline]
    pub fn get_mut(&mut self, id: ArchetypeId) -> Option<&mut Archetype> {
        self.archetypes.get_mut(id.index())
    }

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

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &Archetype> {
        self.archetypes.iter()
    }

    /// Gets the archetype id matching the given inputs or inserts a new one if it doesn't exist.
    /// `table_components` and `sparse_set_components` must be sorted
    ///
    /// # Safety
    /// TableId must exist in tables
    pub(crate) fn get_id_or_insert(
        &mut self,
        table_id: TableId,
        table_components: Vec<ComponentId>,
        sparse_set_components: Vec<ComponentId>,
    ) -> ArchetypeId {
        let table_components = Cow::from(table_components);
        let sparse_set_components = Cow::from(sparse_set_components);
        let archetype_identity = ArchetypeIdentity {
            sparse_set_components: sparse_set_components.clone(),
            table_components: table_components.clone(),
        };

        let archetypes = &mut self.archetypes;
        let archetype_component_count = &mut self.archetype_component_count;
        let mut next_archetype_component_id = move || {
            let id = ArchetypeComponentId(*archetype_component_count);
            *archetype_component_count += 1;
            id
        };
        *self
            .archetype_ids
            .entry(archetype_identity)
            .or_insert_with(move || {
                let id = ArchetypeId(archetypes.len());
                let table_archetype_components = (0..table_components.len())
                    .map(|_| next_archetype_component_id())
                    .collect();
                let sparse_set_archetype_components = (0..sparse_set_components.len())
                    .map(|_| next_archetype_component_id())
                    .collect();
                archetypes.push(Archetype::new(
                    id,
                    table_id,
                    table_components,
                    sparse_set_components,
                    table_archetype_components,
                    sparse_set_archetype_components,
                ));
                id
            })
    }

    #[inline]
    pub fn archetype_components_len(&self) -> usize {
        self.archetype_component_count
    }

    pub fn clear_entities(&mut self) {
        for archetype in self.archetypes.iter_mut() {
            archetype.clear_entities();
        }
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

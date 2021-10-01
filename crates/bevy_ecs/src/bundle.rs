//! Types for handling [`Bundle`]s.
//!
//! This module contains the `Bundle` trait and some other helper types.

pub use bevy_ecs_macros::Bundle;

use crate::{
    archetype::{AddBundle, Archetype, ArchetypeId, Archetypes, ComponentStatus},
    component::{Component, ComponentId, ComponentTicks, Components, StorageType},
    entity::{Entities, Entity, EntityLocation},
    storage::{SparseSetIndex, SparseSets, Storages, Table},
};
use bevy_ecs_macros::all_tuples;
use std::{any::TypeId, collections::HashMap};

/// An ordered collection of components.
///
/// Commonly used for spawning entities and adding and removing components in bulk. This
/// trait is automatically implemented for tuples of components: `(ComponentA, ComponentB)`
/// is a very convenient shorthand when working with one-off collections of components. Note
/// that both the unit type `()` and `(ComponentA, )` are valid bundles. The unit bundle is
/// particularly useful for spawning multiple empty entities by using
/// [`Commands::spawn_batch`](crate::system::Commands::spawn_batch).
///
/// # Examples
///
/// Typically, you will simply use `#[derive(Bundle)]` when creating your own `Bundle`. Each
/// struct field is a component:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)]
/// # struct ComponentA;
/// # #[derive(Component)]
/// # struct ComponentB;
/// # #[derive(Component)]
/// # struct ComponentC;
/// #
/// #[derive(Bundle)]
/// struct MyBundle {
///     a: ComponentA,
///     b: ComponentB,
///     c: ComponentC,
/// }
/// ```
///
/// You can nest bundles using the `#[bundle]` attribute:
/// ```
/// # use bevy_ecs::{component::Component, bundle::Bundle};
///
/// #[derive(Component)]
/// struct X(i32);
/// #[derive(Component)]
/// struct Y(u64);
/// #[derive(Component)]
/// struct Z(String);
///
/// #[derive(Bundle)]
/// struct A {
///     x: X,
///     y: Y,
/// }
///
/// #[derive(Bundle)]
/// struct B {
///     #[bundle]
///     a: A,
///     z: Z,
/// }
/// ```
///
/// # Safety
///
/// - [Bundle::component_ids] must return the ComponentId for each component type in the bundle, in the
///   _exact_ order that [Bundle::get_components] is called.
/// - [Bundle::from_components] must call `func` exactly once for each [ComponentId] returned by
///   [Bundle::component_ids].
pub unsafe trait Bundle: Send + Sync + 'static {
    /// Gets this [Bundle]'s component ids, in the order of this bundle's Components
    fn component_ids(components: &mut Components, storages: &mut Storages) -> Vec<ComponentId>;

    /// Calls `func`, which should return data for each component in the bundle, in the order of
    /// this bundle's Components
    ///
    /// # Safety
    /// Caller must return data for each component in the bundle, in the order of this bundle's
    /// Components
    unsafe fn from_components(func: impl FnMut() -> *mut u8) -> Self
    where
        Self: Sized;

    /// Calls `func` on each value, in the order of this bundle's Components. This will
    /// "mem::forget" the bundle fields, so callers are responsible for dropping the fields if
    /// that is desirable.
    fn get_components(self, func: impl FnMut(*mut u8));
}

macro_rules! tuple_impl {
    ($($name: ident),*) => {
        /// SAFE: Component is returned in tuple-order. [Bundle::from_components] and [Bundle::get_components] use tuple-order
        unsafe impl<$($name: Component),*> Bundle for ($($name,)*) {
            #[allow(unused_variables)]
            fn component_ids(components: &mut Components, storages: &mut Storages) -> Vec<ComponentId> {
                vec![$(components.init_component::<$name>(storages)),*]
            }

            #[allow(unused_variables, unused_mut)]
            #[allow(clippy::unused_unit)]
            unsafe fn from_components(mut func: impl FnMut() -> *mut u8) -> Self {
                #[allow(non_snake_case)]
                let ($(mut $name,)*) = (
                    $(func().cast::<$name>(),)*
                );
                ($($name.read(),)*)
            }

            #[allow(unused_variables, unused_mut)]
            fn get_components(self, mut func: impl FnMut(*mut u8)) {
                #[allow(non_snake_case)]
                let ($(mut $name,)*) = self;
                $(
                    func((&mut $name as *mut $name).cast::<u8>());
                    std::mem::forget($name);
                )*
            }
        }
    }
}

all_tuples!(tuple_impl, 0, 15, C);

#[derive(Debug, Clone, Copy)]
pub struct BundleId(usize);

impl BundleId {
    #[inline]
    pub fn index(self) -> usize {
        self.0
    }
}

impl SparseSetIndex for BundleId {
    #[inline]
    fn sparse_set_index(&self) -> usize {
        self.index()
    }

    fn get_sparse_set_index(value: usize) -> Self {
        Self(value)
    }
}

pub struct BundleInfo {
    pub(crate) id: BundleId,
    pub(crate) component_ids: Vec<ComponentId>,
    pub(crate) storage_types: Vec<StorageType>,
}

impl BundleInfo {
    #[inline]
    pub fn id(&self) -> BundleId {
        self.id
    }

    #[inline]
    pub fn components(&self) -> &[ComponentId] {
        &self.component_ids
    }

    #[inline]
    pub fn storage_types(&self) -> &[StorageType] {
        &self.storage_types
    }

    pub(crate) fn get_bundle_inserter<'a, 'b>(
        &'b self,
        entities: &'a mut Entities,
        archetypes: &'a mut Archetypes,
        components: &mut Components,
        storages: &'a mut Storages,
        archetype_id: ArchetypeId,
        change_tick: u32,
    ) -> BundleInserter<'a, 'b> {
        let new_archetype_id =
            self.add_bundle_to_archetype(archetypes, storages, components, archetype_id);
        let archetypes_ptr = archetypes.archetypes.as_mut_ptr();
        if new_archetype_id == archetype_id {
            let archetype = &mut archetypes[archetype_id];
            let table_id = archetype.table_id();
            BundleInserter {
                bundle_info: self,
                archetype,
                entities,
                sparse_sets: &mut storages.sparse_sets,
                table: &mut storages.tables[table_id],
                archetypes_ptr,
                change_tick,
                result: InsertBundleResult::SameArchetype,
            }
        } else {
            let (archetype, new_archetype) = archetypes.get_2_mut(archetype_id, new_archetype_id);
            let table_id = archetype.table_id();
            if table_id == new_archetype.table_id() {
                BundleInserter {
                    bundle_info: self,
                    archetype,
                    archetypes_ptr,
                    entities,
                    sparse_sets: &mut storages.sparse_sets,
                    table: &mut storages.tables[table_id],
                    change_tick,
                    result: InsertBundleResult::NewArchetypeSameTable { new_archetype },
                }
            } else {
                let (table, new_table) = storages
                    .tables
                    .get_2_mut(table_id, new_archetype.table_id());
                BundleInserter {
                    bundle_info: self,
                    archetype,
                    sparse_sets: &mut storages.sparse_sets,
                    entities,
                    archetypes_ptr,
                    table,
                    change_tick,
                    result: InsertBundleResult::NewArchetypeNewTable {
                        new_archetype,
                        new_table,
                    },
                }
            }
        }
    }

    pub(crate) fn get_bundle_spawner<'a, 'b>(
        &'b self,
        entities: &'a mut Entities,
        archetypes: &'a mut Archetypes,
        components: &mut Components,
        storages: &'a mut Storages,
        change_tick: u32,
    ) -> BundleSpawner<'a, 'b> {
        let new_archetype_id =
            self.add_bundle_to_archetype(archetypes, storages, components, ArchetypeId::EMPTY);
        let (empty_archetype, archetype) =
            archetypes.get_2_mut(ArchetypeId::EMPTY, new_archetype_id);
        let table = &mut storages.tables[archetype.table_id()];
        let add_bundle = empty_archetype.edges().get_add_bundle(self.id()).unwrap();
        BundleSpawner {
            archetype,
            add_bundle,
            bundle_info: self,
            table,
            entities,
            sparse_sets: &mut storages.sparse_sets,
            change_tick,
        }
    }

    /// # Safety
    /// `table` must be the "new" table for `entity`. `table_row` must have space allocated for the `entity`, `bundle` must match this BundleInfo's type
    #[inline]
    #[allow(clippy::too_many_arguments)]
    unsafe fn write_components<T: Bundle>(
        &self,
        table: &mut Table,
        sparse_sets: &mut SparseSets,
        add_bundle: &AddBundle,
        entity: Entity,
        table_row: usize,
        change_tick: u32,
        bundle: T,
    ) {
        // NOTE: get_components calls this closure on each component in "bundle order".
        // bundle_info.component_ids are also in "bundle order"
        let mut bundle_component = 0;
        bundle.get_components(|component_ptr| {
            let component_id = *self.component_ids.get_unchecked(bundle_component);
            match self.storage_types[bundle_component] {
                StorageType::Table => {
                    let column = table.get_column_mut(component_id).unwrap();
                    match add_bundle.bundle_status.get_unchecked(bundle_component) {
                        ComponentStatus::Added => {
                            column.initialize(
                                table_row,
                                component_ptr,
                                ComponentTicks::new(change_tick),
                            );
                        }
                        ComponentStatus::Mutated => {
                            column.replace(table_row, component_ptr, change_tick);
                        }
                    }
                }
                StorageType::SparseSet => {
                    let sparse_set = sparse_sets.get_mut(component_id).unwrap();
                    sparse_set.insert(entity, component_ptr, change_tick);
                }
            }
            bundle_component += 1;
        });
    }

    /// Adds a bundle to the given archetype and returns the resulting archetype. This could be the same
    /// [ArchetypeId], in the event that adding the given bundle does not result in an Archetype change.
    /// Results are cached in the Archetype Graph to avoid redundant work.
    pub(crate) fn add_bundle_to_archetype(
        &self,
        archetypes: &mut Archetypes,
        storages: &mut Storages,
        components: &mut Components,
        archetype_id: ArchetypeId,
    ) -> ArchetypeId {
        if let Some(add_bundle) = archetypes[archetype_id].edges().get_add_bundle(self.id) {
            return add_bundle.archetype_id;
        }
        let mut new_table_components = Vec::new();
        let mut new_sparse_set_components = Vec::new();
        let mut bundle_status = Vec::with_capacity(self.component_ids.len());

        let current_archetype = &mut archetypes[archetype_id];
        for component_id in self.component_ids.iter().cloned() {
            if current_archetype.contains(component_id) {
                bundle_status.push(ComponentStatus::Mutated);
            } else {
                bundle_status.push(ComponentStatus::Added);
                // SAFE: component_id exists
                let component_info = unsafe { components.get_info_unchecked(component_id) };
                match component_info.storage_type() {
                    StorageType::Table => new_table_components.push(component_id),
                    StorageType::SparseSet => new_sparse_set_components.push(component_id),
                }
            }
        }

        if new_table_components.is_empty() && new_sparse_set_components.is_empty() {
            let edges = current_archetype.edges_mut();
            // the archetype does not change when we add this bundle
            edges.insert_add_bundle(self.id, archetype_id, bundle_status);
            archetype_id
        } else {
            let table_id;
            let table_components;
            let sparse_set_components;
            // the archetype changes when we add this bundle. prepare the new archetype and storages
            {
                let current_archetype = &archetypes[archetype_id];
                table_components = if new_table_components.is_empty() {
                    // if there are no new table components, we can keep using this table
                    table_id = current_archetype.table_id();
                    current_archetype.table_components().to_vec()
                } else {
                    new_table_components.extend(current_archetype.table_components());
                    // sort to ignore order while hashing
                    new_table_components.sort();
                    // SAFE: all component ids in `new_table_components` exist
                    table_id = unsafe {
                        storages
                            .tables
                            .get_id_or_insert(&new_table_components, components)
                    };

                    new_table_components
                };

                sparse_set_components = if new_sparse_set_components.is_empty() {
                    current_archetype.sparse_set_components().to_vec()
                } else {
                    new_sparse_set_components.extend(current_archetype.sparse_set_components());
                    // sort to ignore order while hashing
                    new_sparse_set_components.sort();
                    new_sparse_set_components
                };
            };
            let new_archetype_id =
                archetypes.get_id_or_insert(table_id, table_components, sparse_set_components);
            // add an edge from the old archetype to the new archetype
            archetypes[archetype_id].edges_mut().insert_add_bundle(
                self.id,
                new_archetype_id,
                bundle_status,
            );
            new_archetype_id
        }
    }
}

pub(crate) struct BundleInserter<'a, 'b> {
    pub(crate) archetype: &'a mut Archetype,
    pub(crate) entities: &'a mut Entities,
    bundle_info: &'b BundleInfo,
    table: &'a mut Table,
    sparse_sets: &'a mut SparseSets,
    result: InsertBundleResult<'a>,
    archetypes_ptr: *mut Archetype,
    change_tick: u32,
}

pub(crate) enum InsertBundleResult<'a> {
    SameArchetype,
    NewArchetypeSameTable {
        new_archetype: &'a mut Archetype,
    },
    NewArchetypeNewTable {
        new_archetype: &'a mut Archetype,
        new_table: &'a mut Table,
    },
}

impl<'a, 'b> BundleInserter<'a, 'b> {
    /// # Safety
    /// `entity` must currently exist in the source archetype for this inserter. `archetype_index` must be `entity`'s location in the archetype.
    /// `T` must match this BundleInfo's type
    #[inline]
    pub unsafe fn insert<T: Bundle>(
        &mut self,
        entity: Entity,
        archetype_index: usize,
        bundle: T,
    ) -> EntityLocation {
        let location = EntityLocation {
            index: archetype_index,
            archetype_id: self.archetype.id(),
        };
        match &mut self.result {
            InsertBundleResult::SameArchetype => {
                // PERF: this could be looked up during Inserter construction and stored (but borrowing makes this nasty)
                let add_bundle = self
                    .archetype
                    .edges()
                    .get_add_bundle(self.bundle_info.id)
                    .unwrap();
                self.bundle_info.write_components(
                    self.table,
                    self.sparse_sets,
                    add_bundle,
                    entity,
                    self.archetype.entity_table_row(archetype_index),
                    self.change_tick,
                    bundle,
                );
                location
            }
            InsertBundleResult::NewArchetypeSameTable { new_archetype } => {
                let result = self.archetype.swap_remove(location.index);
                if let Some(swapped_entity) = result.swapped_entity {
                    self.entities.meta[swapped_entity.id as usize].location = location;
                }
                let new_location = new_archetype.allocate(entity, result.table_row);
                self.entities.meta[entity.id as usize].location = new_location;

                // PERF: this could be looked up during Inserter construction and stored (but borrowing makes this nasty)
                let add_bundle = self
                    .archetype
                    .edges()
                    .get_add_bundle(self.bundle_info.id)
                    .unwrap();
                self.bundle_info.write_components(
                    self.table,
                    self.sparse_sets,
                    add_bundle,
                    entity,
                    result.table_row,
                    self.change_tick,
                    bundle,
                );
                new_location
            }
            InsertBundleResult::NewArchetypeNewTable {
                new_archetype,
                new_table,
            } => {
                let result = self.archetype.swap_remove(location.index);
                if let Some(swapped_entity) = result.swapped_entity {
                    self.entities.meta[swapped_entity.id as usize].location = location;
                }
                // PERF: store "non bundle" components in edge, then just move those to avoid
                // redundant copies
                let move_result = self
                    .table
                    .move_to_superset_unchecked(result.table_row, &mut *new_table);
                let new_location = new_archetype.allocate(entity, move_result.new_row);
                self.entities.meta[entity.id as usize].location = new_location;

                // if an entity was moved into this entity's table spot, update its table row
                if let Some(swapped_entity) = move_result.swapped_entity {
                    let swapped_location = self.entities.get(swapped_entity).unwrap();
                    let swapped_archetype = if self.archetype.id() == swapped_location.archetype_id
                    {
                        &mut *self.archetype
                    } else if new_archetype.id() == swapped_location.archetype_id {
                        &mut *new_archetype
                    } else {
                        // SAFE: the only two borrowed archetypes are above and we just did collision checks
                        &mut *self
                            .archetypes_ptr
                            .add(swapped_location.archetype_id.index())
                    };

                    swapped_archetype
                        .set_entity_table_row(swapped_location.index, result.table_row);
                }

                // PERF: this could be looked up during Inserter construction and stored (but borrowing makes this nasty)
                let add_bundle = self
                    .archetype
                    .edges()
                    .get_add_bundle(self.bundle_info.id)
                    .unwrap();
                self.bundle_info.write_components(
                    new_table,
                    self.sparse_sets,
                    add_bundle,
                    entity,
                    move_result.new_row,
                    self.change_tick,
                    bundle,
                );
                new_location
            }
        }
    }
}

pub(crate) struct BundleSpawner<'a, 'b> {
    pub(crate) archetype: &'a mut Archetype,
    pub(crate) entities: &'a mut Entities,
    add_bundle: &'a AddBundle,
    bundle_info: &'b BundleInfo,
    table: &'a mut Table,
    sparse_sets: &'a mut SparseSets,
    change_tick: u32,
}

impl<'a, 'b> BundleSpawner<'a, 'b> {
    pub fn reserve_storage(&mut self, additional: usize) {
        self.archetype.reserve(additional);
        self.table.reserve(additional);
    }
    /// # Safety
    /// `entity` must be allocated (but non existent), `T` must match this BundleInfo's type
    #[inline]
    pub unsafe fn spawn_non_existent<T: Bundle>(
        &mut self,
        entity: Entity,
        bundle: T,
    ) -> EntityLocation {
        let table_row = self.table.allocate(entity);
        let location = self.archetype.allocate(entity, table_row);
        self.bundle_info.write_components(
            self.table,
            self.sparse_sets,
            self.add_bundle,
            entity,
            table_row,
            self.change_tick,
            bundle,
        );
        self.entities.meta[entity.id as usize].location = location;

        location
    }

    /// # Safety
    /// `T` must match this BundleInfo's type
    #[inline]
    pub unsafe fn spawn<T: Bundle>(&mut self, bundle: T) -> Entity {
        let entity = self.entities.alloc();
        // SAFE: entity is allocated (but non-existent), `T` matches this BundleInfo's type
        self.spawn_non_existent(entity, bundle);
        entity
    }
}

#[derive(Default)]
pub struct Bundles {
    bundle_infos: Vec<BundleInfo>,
    bundle_ids: HashMap<TypeId, BundleId>,
}

impl Bundles {
    #[inline]
    pub fn get(&self, bundle_id: BundleId) -> Option<&BundleInfo> {
        self.bundle_infos.get(bundle_id.index())
    }

    #[inline]
    pub fn get_id(&self, type_id: TypeId) -> Option<BundleId> {
        self.bundle_ids.get(&type_id).cloned()
    }

    pub(crate) fn init_info<'a, T: Bundle>(
        &'a mut self,
        components: &mut Components,
        storages: &mut Storages,
    ) -> &'a BundleInfo {
        let bundle_infos = &mut self.bundle_infos;
        let id = self.bundle_ids.entry(TypeId::of::<T>()).or_insert_with(|| {
            let component_ids = T::component_ids(components, storages);
            let id = BundleId(bundle_infos.len());
            // SAFE: T::component_id ensures info was created
            let bundle_info = unsafe {
                initialize_bundle(std::any::type_name::<T>(), component_ids, id, components)
            };
            bundle_infos.push(bundle_info);
            id
        });
        // SAFE: index either exists, or was initialized
        unsafe { self.bundle_infos.get_unchecked(id.0) }
    }
}

/// # Safety
///
/// `component_id` must be valid [ComponentId]'s
unsafe fn initialize_bundle(
    bundle_type_name: &'static str,
    component_ids: Vec<ComponentId>,
    id: BundleId,
    components: &mut Components,
) -> BundleInfo {
    let mut storage_types = Vec::new();

    for &component_id in &component_ids {
        // SAFE: component_id exists and is therefore valid
        let component_info = components.get_info_unchecked(component_id);
        storage_types.push(component_info.storage_type());
    }

    let mut deduped = component_ids.clone();
    deduped.sort();
    deduped.dedup();
    if deduped.len() != component_ids.len() {
        panic!("Bundle {} has duplicate components", bundle_type_name);
    }

    BundleInfo {
        id,
        component_ids,
        storage_types,
    }
}

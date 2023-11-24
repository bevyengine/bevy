//! Types for handling [`Bundle`]s.
//!
//! This module contains the [`Bundle`] trait and some other helper types.

pub use bevy_ecs_macros::Bundle;
use bevy_utils::{HashMap, HashSet};

use crate::{
    archetype::{
        AddBundle, Archetype, ArchetypeId, Archetypes, BundleComponentStatus, ComponentStatus,
        SpawnBundleStatus,
    },
    component::{Component, ComponentId, ComponentStorage, Components, StorageType, Tick},
    entity::{Entities, Entity, EntityLocation},
    prelude::World,
    query::DebugCheckedUnwrap,
    storage::{SparseSetIndex, SparseSets, Storages, Table, TableRow},
    TypeIdMap,
};
use bevy_ptr::OwningPtr;
use bevy_utils::all_tuples;
use std::any::TypeId;

/// The `Bundle` trait enables insertion and removal of [`Component`]s from an entity.
///
/// Implementors of the `Bundle` trait are called 'bundles'.
///
/// Each bundle represents a static set of [`Component`] types.
/// Currently, bundles can only contain one of each [`Component`], and will
/// panic once initialised if this is not met.
///
/// ## Insertion
///
/// The primary use for bundles is to add a useful collection of components to an entity.
///
/// Adding a value of bundle to an entity will add the components from the set it
/// represents to the entity.
/// The values of these components are taken from the bundle.
/// If an entity already had one of these components, the entity's original component value
/// will be overwritten.
///
/// Importantly, bundles are only their constituent set of components.
/// You **should not** use bundles as a unit of behavior.
/// The behavior of your app can only be considered in terms of components, as systems,
/// which drive the behavior of a `bevy` application, operate on combinations of
/// components.
///
/// This rule is also important because multiple bundles may contain the same component type,
/// calculated in different ways &mdash; adding both of these bundles to one entity
/// would create incoherent behavior.
/// This would be unexpected if bundles were treated as an abstraction boundary, as
/// the abstraction would be unmaintainable for these cases.
/// For example, both `Camera3dBundle` and `Camera2dBundle` contain the `CameraRenderGraph`
/// component, but specifying different render graphs to use.
/// If the bundles were both added to the same entity, only one of these two bundles would work.
///
/// For this reason, there is intentionally no [`Query`] to match whether an entity
/// contains the components of a bundle.
/// Queries should instead only select the components they logically operate on.
///
/// ## Removal
///
/// Bundles are also used when removing components from an entity.
///
/// Removing a bundle from an entity will remove any of its components attached
/// to the entity from the entity.
/// That is, if the entity does not have all the components of the bundle, those
/// which are present will be removed.
///
/// # Implementors
///
/// Every type which implements [`Component`] also implements `Bundle`, since
/// [`Component`] types can be added to or removed from an entity.
///
/// Additionally, [Tuples](`tuple`) of bundles are also [`Bundle`] (with up to 15 bundles).
/// These bundles contain the items of the 'inner' bundles.
/// This is a convenient shorthand which is primarily used when spawning entities.
/// For example, spawning an entity using the bundle `(SpriteBundle {...}, PlayerMarker)`
/// will spawn an entity with components required for a 2d sprite, and the `PlayerMarker` component.
///
/// [`unit`], otherwise known as [`()`](`unit`), is a [`Bundle`] containing no components (since it
/// can also be considered as the empty tuple).
/// This can be useful for spawning large numbers of empty entities using
/// [`World::spawn_batch`](crate::world::World::spawn_batch).
///
/// Tuple bundles can be nested, which can be used to create an anonymous bundle with more than
/// 15 items.
/// However, in most cases where this is required, the derive macro [`derive@Bundle`] should be
/// used instead.
/// The derived `Bundle` implementation contains the items of its fields, which all must
/// implement `Bundle`.
/// As explained above, this includes any [`Component`] type, and other derived bundles.
///
/// If you want to add `PhantomData` to your `Bundle` you have to mark it with `#[bundle(ignore)]`.
/// ```
/// # use std::marker::PhantomData;
/// use bevy_ecs::{component::Component, bundle::Bundle};
///
/// #[derive(Component)]
/// struct XPosition(i32);
/// #[derive(Component)]
/// struct YPosition(i32);
///
/// #[derive(Bundle)]
/// struct PositionBundle {
///     // A bundle can contain components
///     x: XPosition,
///     y: YPosition,
/// }
///
/// // You have to implement `Default` for ignored field types in bundle structs.
/// #[derive(Default)]
/// struct Other(f32);
///
/// #[derive(Bundle)]
/// struct NamedPointBundle<T: Send + Sync + 'static> {
///     // Or other bundles
///     a: PositionBundle,
///     // In addition to more components
///     z: PointName,
///
///     // when you need to use `PhantomData` you have to mark it as ignored
///     #[bundle(ignore)]
///     _phantom_data: PhantomData<T>
/// }
///
/// #[derive(Component)]
/// struct PointName(String);
/// ```
///
/// # Safety
///
/// Manual implementations of this trait are unsupported.
/// That is, there is no safe way to implement this trait, and you must not do so.
/// If you want a type to implement [`Bundle`], you must use [`derive@Bundle`](derive@Bundle).
///
/// [`Query`]: crate::system::Query
// Some safety points:
// - [`Bundle::component_ids`] must return the [`ComponentId`] for each component type in the
// bundle, in the _exact_ order that [`DynamicBundle::get_components`] is called.
// - [`Bundle::from_components`] must call `func` exactly once for each [`ComponentId`] returned by
//   [`Bundle::component_ids`].
pub unsafe trait Bundle: DynamicBundle + Send + Sync + 'static {
    /// Gets this [`Bundle`]'s component ids, in the order of this bundle's [`Component`]s
    #[doc(hidden)]
    fn component_ids(
        components: &mut Components,
        storages: &mut Storages,
        ids: &mut impl FnMut(ComponentId),
    );

    /// Calls `func`, which should return data for each component in the bundle, in the order of
    /// this bundle's [`Component`]s
    ///
    /// # Safety
    /// Caller must return data for each component in the bundle, in the order of this bundle's
    /// [`Component`]s
    #[doc(hidden)]
    unsafe fn from_components<T, F>(ctx: &mut T, func: &mut F) -> Self
    where
        // Ensure that the `OwningPtr` is used correctly
        F: for<'a> FnMut(&'a mut T) -> OwningPtr<'a>,
        Self: Sized;
}

/// The parts from [`Bundle`] that don't require statically knowing the components of the bundle.
pub trait DynamicBundle {
    // SAFETY:
    // The `StorageType` argument passed into [`Bundle::get_components`] must be correct for the
    // component being fetched.
    //
    /// Calls `func` on each value, in the order of this bundle's [`Component`]s. This passes
    /// ownership of the component values to `func`.
    #[doc(hidden)]
    fn get_components(self, func: &mut impl FnMut(StorageType, OwningPtr<'_>));
}

// SAFETY:
// - `Bundle::component_ids` calls `ids` for C's component id (and nothing else)
// - `Bundle::get_components` is called exactly once for C and passes the component's storage type based on it's associated constant.
// - `Bundle::from_components` calls `func` exactly once for C, which is the exact value returned by `Bundle::component_ids`.
unsafe impl<C: Component> Bundle for C {
    fn component_ids(
        components: &mut Components,
        storages: &mut Storages,
        ids: &mut impl FnMut(ComponentId),
    ) {
        ids(components.init_component::<C>(storages).id());
    }

    unsafe fn from_components<T, F>(ctx: &mut T, func: &mut F) -> Self
    where
        // Ensure that the `OwningPtr` is used correctly
        F: for<'a> FnMut(&'a mut T) -> OwningPtr<'a>,
        Self: Sized,
    {
        // Safety: The id given in `component_ids` is for `Self`
        func(ctx).read()
    }
}

impl<C: Component> DynamicBundle for C {
    #[inline]
    fn get_components(self, func: &mut impl FnMut(StorageType, OwningPtr<'_>)) {
        OwningPtr::make(self, |ptr| func(C::Storage::STORAGE_TYPE, ptr));
    }
}

macro_rules! tuple_impl {
    ($($name: ident),*) => {
        // SAFETY:
        // - `Bundle::component_ids` calls `ids` for each component type in the
        // bundle, in the exact order that `DynamicBundle::get_components` is called.
        // - `Bundle::from_components` calls `func` exactly once for each `ComponentId` returned by `Bundle::component_ids`.
        // - `Bundle::get_components` is called exactly once for each member. Relies on the above implementation to pass the correct
        //   `StorageType` into the callback.
        unsafe impl<$($name: Bundle),*> Bundle for ($($name,)*) {
            #[allow(unused_variables)]
            fn component_ids(components: &mut Components, storages: &mut Storages, ids: &mut impl FnMut(ComponentId)){
                $(<$name as Bundle>::component_ids(components, storages, ids);)*
            }

            #[allow(unused_variables, unused_mut)]
            #[allow(clippy::unused_unit)]
            unsafe fn from_components<T, F>(ctx: &mut T, func: &mut F) -> Self
            where
                F: FnMut(&mut T) -> OwningPtr<'_>
            {
                // Rust guarantees that tuple calls are evaluated 'left to right'.
                // https://doc.rust-lang.org/reference/expressions.html#evaluation-order-of-operands
                ($(<$name as Bundle>::from_components(ctx, func),)*)
            }
        }

        impl<$($name: Bundle),*> DynamicBundle for ($($name,)*) {
            #[allow(unused_variables, unused_mut)]
            #[inline(always)]
            fn get_components(self, func: &mut impl FnMut(StorageType, OwningPtr<'_>)) {
                #[allow(non_snake_case)]
                let ($(mut $name,)*) = self;
                $(
                    $name.get_components(&mut *func);
                )*
            }
        }
    }
}

all_tuples!(tuple_impl, 0, 15, B);

/// For a specific [`World`], this stores a unique value identifying a type of a registered [`Bundle`].
///
/// [`World`]: crate::world::World
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct BundleId(usize);

impl BundleId {
    /// Returns the index of the associated [`Bundle`] type.
    ///
    /// Note that this is unique per-world, and should not be reused across them.
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

    #[inline]
    fn get_sparse_set_index(value: usize) -> Self {
        Self(value)
    }
}

/// Stores metadata associated with a specific type of [`Bundle`] for a given [`World`].
///
/// [`World`]: crate::world::World
pub struct BundleInfo {
    id: BundleId,
    // SAFETY: Every ID in this list must be valid within the World that owns the BundleInfo,
    // must have its storage initialized (i.e. columns created in tables, sparse set created),
    // and must be in the same order as the source bundle type writes its components in.
    component_ids: Vec<ComponentId>,
}

impl BundleInfo {
    /// Create a new [`BundleInfo`].
    ///
    /// # Safety
    ///
    // Every ID in `component_ids` must be valid within the World that owns the BundleInfo,
    // must have its storage initialized (i.e. columns created in tables, sparse set created),
    // and must be in the same order as the source bundle type writes its components in.
    unsafe fn new(
        bundle_type_name: &'static str,
        components: &Components,
        component_ids: Vec<ComponentId>,
        id: BundleId,
    ) -> BundleInfo {
        let mut deduped = component_ids.clone();
        deduped.sort();
        deduped.dedup();

        if deduped.len() != component_ids.len() {
            // TODO: Replace with `Vec::partition_dedup` once https://github.com/rust-lang/rust/issues/54279 is stabilized
            let mut seen = HashSet::new();
            let mut dups = Vec::new();
            for id in component_ids {
                if !seen.insert(id) {
                    dups.push(id);
                }
            }

            let names = dups
                .into_iter()
                .map(|id| {
                    // SAFETY: the caller ensures component_id is valid.
                    unsafe { components.get_info_unchecked(id).name() }
                })
                .collect::<Vec<_>>()
                .join(", ");

            panic!("Bundle {bundle_type_name} has duplicate components: {names}");
        }

        // SAFETY: The caller ensures that component_ids:
        // - is valid for the associated world
        // - has had its storage initialized
        // - is in the same order as the source bundle type
        BundleInfo { id, component_ids }
    }

    /// Returns a value identifying the associated [`Bundle`] type.
    #[inline]
    pub const fn id(&self) -> BundleId {
        self.id
    }

    /// Returns the [ID](ComponentId) of each component stored in this bundle.
    #[inline]
    pub fn components(&self) -> &[ComponentId] {
        &self.component_ids
    }

    /// This writes components from a given [`Bundle`] to the given entity.
    ///
    /// # Safety
    ///
    /// `bundle_component_status` must return the "correct" [`ComponentStatus`] for each component
    /// in the [`Bundle`], with respect to the entity's original archetype (prior to the bundle being added)
    /// For example, if the original archetype already has `ComponentA` and `T` also has `ComponentA`, the status
    /// should be `Mutated`. If the original archetype does not have `ComponentA`, the status should be `Added`.
    /// When "inserting" a bundle into an existing entity, [`AddBundle`](crate::archetype::AddBundle)
    /// should be used, which will report `Added` vs `Mutated` status based on the current archetype's structure.
    /// When spawning a bundle, [`SpawnBundleStatus`] can be used instead, which removes the need
    /// to look up the [`AddBundle`](crate::archetype::AddBundle) in the archetype graph, which requires
    /// ownership of the entity's current archetype.
    ///
    /// `table` must be the "new" table for `entity`. `table_row` must have space allocated for the
    /// `entity`, `bundle` must match this [`BundleInfo`]'s type
    #[inline]
    #[allow(clippy::too_many_arguments)]
    unsafe fn write_components<T: DynamicBundle, S: BundleComponentStatus>(
        &self,
        table: &mut Table,
        sparse_sets: &mut SparseSets,
        bundle_component_status: &S,
        entity: Entity,
        table_row: TableRow,
        change_tick: Tick,
        bundle: T,
    ) {
        // NOTE: get_components calls this closure on each component in "bundle order".
        // bundle_info.component_ids are also in "bundle order"
        let mut bundle_component = 0;
        bundle.get_components(&mut |storage_type, component_ptr| {
            let component_id = *self.component_ids.get_unchecked(bundle_component);
            match storage_type {
                StorageType::Table => {
                    let column =
                        // SAFETY: If component_id is in self.component_ids, BundleInfo::new requires that
                        // the target table contains the component.
                        unsafe { table.get_column_mut(component_id).debug_checked_unwrap() };
                    // SAFETY: bundle_component is a valid index for this bundle
                    match bundle_component_status.get_status(bundle_component) {
                        ComponentStatus::Added => {
                            column.initialize(table_row, component_ptr, change_tick);
                        }
                        ComponentStatus::Mutated => {
                            column.replace(table_row, component_ptr, change_tick);
                        }
                    }
                }
                StorageType::SparseSet => {
                    let sparse_set =
                        // SAFETY: If component_id is in self.component_ids, BundleInfo::new requires that
                        // a sparse set exists for the component.
                        unsafe { sparse_sets.get_mut(component_id).debug_checked_unwrap() };
                    sparse_set.insert(entity, component_ptr, change_tick);
                }
            }
            bundle_component += 1;
        });
    }

    /// Adds a bundle to the given archetype and returns the resulting archetype. This could be the
    /// same [`ArchetypeId`], in the event that adding the given bundle does not result in an
    /// [`Archetype`] change. Results are cached in the [`Archetype`] graph to avoid redundant work.
    pub(crate) fn add_bundle_to_archetype(
        &self,
        archetypes: &mut Archetypes,
        storages: &mut Storages,
        components: &Components,
        archetype_id: ArchetypeId,
    ) -> ArchetypeId {
        if let Some(add_bundle_id) = archetypes[archetype_id].edges().get_add_bundle(self.id) {
            return add_bundle_id;
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
                // SAFETY: component_id exists
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
                    current_archetype.table_components().collect()
                } else {
                    new_table_components.extend(current_archetype.table_components());
                    // sort to ignore order while hashing
                    new_table_components.sort();
                    // SAFETY: all component ids in `new_table_components` exist
                    table_id = unsafe {
                        storages
                            .tables
                            .get_id_or_insert(&new_table_components, components)
                    };

                    new_table_components
                };

                sparse_set_components = if new_sparse_set_components.is_empty() {
                    current_archetype.sparse_set_components().collect()
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

pub(crate) struct BundleInserter<'w> {
    world: &'w mut World,
    archetype: *mut Archetype,
    table: *mut Table,
    bundle_info: *const BundleInfo,
    add_bundle: *const AddBundle,
    result: InsertBundleResult,
    change_tick: Tick,
}

pub(crate) enum InsertBundleResult {
    SameArchetype,
    NewArchetypeSameTable {
        new_archetype: *mut Archetype,
    },
    NewArchetypeNewTable {
        new_archetype: *mut Archetype,
        new_table: *mut Table,
    },
}

impl<'w> BundleInserter<'w> {
    #[inline]
    pub fn new<T: Bundle>(
        world: &'w mut World,
        archetype_id: ArchetypeId,
        change_tick: Tick,
    ) -> Self {
        let bundle_info: *const BundleInfo = world
            .bundles
            .init_info::<T>(&mut world.components, &mut world.storages);
        unsafe { Self::new_with_info(world, archetype_id, bundle_info, change_tick) }
    }

    #[inline]
    pub(crate) unsafe fn new_with_info(
        world: &'w mut World,
        archetype_id: ArchetypeId,
        bundle_info: *const BundleInfo,
        change_tick: Tick,
    ) -> Self {
        let bundle_info = &*bundle_info;
        let bundle_id = bundle_info.id();
        let new_archetype_id = bundle_info.add_bundle_to_archetype(
            &mut world.archetypes,
            &mut world.storages,
            &world.components,
            archetype_id,
        );
        if new_archetype_id == archetype_id {
            let archetype = &mut world.archetypes[archetype_id];
            let table_id = archetype.table_id();
            let add_bundle: *const AddBundle = unsafe {
                archetype
                    .edges()
                    .get_add_bundle_internal(bundle_id)
                    .debug_checked_unwrap()
            };
            let table: *mut Table = &mut world.storages.tables[table_id];
            let archetype: *mut Archetype = archetype;
            Self {
                world,
                archetype,
                table,
                bundle_info,
                add_bundle,
                result: InsertBundleResult::SameArchetype,
                change_tick,
            }
        } else {
            let (archetype, new_archetype) =
                world.archetypes.get_2_mut(archetype_id, new_archetype_id);
            let table_id = archetype.table_id();
            let new_table_id = new_archetype.table_id();
            let add_bundle: *const AddBundle = unsafe {
                archetype
                    .edges()
                    .get_add_bundle_internal(bundle_id)
                    .debug_checked_unwrap()
            };
            let table: *mut Table = &mut world.storages.tables[table_id];
            let archetype: *mut Archetype = archetype;
            let new_archetype: *mut Archetype = new_archetype;
            if table_id == new_table_id {
                Self {
                    world,
                    archetype,
                    table,
                    bundle_info,
                    add_bundle,
                    result: InsertBundleResult::NewArchetypeSameTable { new_archetype },
                    change_tick,
                }
            } else {
                let new_table: *mut Table = &mut world.storages.tables[new_table_id];
                Self {
                    world,
                    archetype,
                    table,
                    bundle_info,
                    add_bundle,
                    result: InsertBundleResult::NewArchetypeNewTable {
                        new_archetype,
                        new_table,
                    },
                    change_tick,
                }
            }
        }
    }
    /// # Safety
    /// `entity` must currently exist in the source archetype for this inserter. `archetype_row`
    /// must be `entity`'s location in the archetype. `T` must match this [`BundleInfo`]'s type
    #[inline]
    pub(crate) unsafe fn insert<T: DynamicBundle>(
        &mut self,
        entity: Entity,
        location: EntityLocation,
        bundle: T,
    ) -> EntityLocation {
        let bundle_info = &*self.bundle_info;
        let add_bundle: &AddBundle = &*self.add_bundle;
        let world = &mut *self.world;
        for (i, component_id) in bundle_info.components().iter().cloned().enumerate() {
            let hooks = unsafe { world.components.get_info_unchecked(component_id) }.hooks();
            if let ComponentStatus::Added = add_bundle.bundle_status[i] {
                if let Some(hook) = hooks.on_add {
                    hook(unsafe { world.into_deferred() }, entity)
                }
            }
            if let Some(hook) = hooks.on_insert {
                hook(unsafe { world.into_deferred() }, entity)
            }
        }

        match self.result {
            InsertBundleResult::SameArchetype => {
                bundle_info.write_components(
                    &mut *self.table,
                    &mut world.storages.sparse_sets,
                    add_bundle,
                    entity,
                    location.table_row,
                    self.change_tick,
                    bundle,
                );
                location
            }
            InsertBundleResult::NewArchetypeSameTable { new_archetype } => {
                let table = &mut *self.table;
                let archetype = &mut *self.archetype;
                let new_archetype = &mut *new_archetype;
                let result = archetype.swap_remove(location.archetype_row);
                if let Some(swapped_entity) = result.swapped_entity {
                    let swapped_location =
                        // SAFETY: If the swap was successful, swapped_entity must be valid.
                        unsafe { world.entities.get(swapped_entity).debug_checked_unwrap() };
                    world.entities.set(
                        swapped_entity.index(),
                        EntityLocation {
                            archetype_id: swapped_location.archetype_id,
                            archetype_row: location.archetype_row,
                            table_id: swapped_location.table_id,
                            table_row: swapped_location.table_row,
                        },
                    );
                }
                let new_location = new_archetype.allocate(entity, result.table_row);
                world.entities.set(entity.index(), new_location);
                bundle_info.write_components(
                    table,
                    &mut world.storages.sparse_sets,
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
                let table = &mut *self.table;
                let new_table = &mut *new_table;
                let archetype = &mut *self.archetype;
                let new_archetype = &mut *new_archetype;
                let result = archetype.swap_remove(location.archetype_row);
                if let Some(swapped_entity) = result.swapped_entity {
                    let swapped_location =
                        // SAFETY: If the swap was successful, swapped_entity must be valid.
                        unsafe { world.entities.get(swapped_entity).debug_checked_unwrap() };
                    world.entities.set(
                        swapped_entity.index(),
                        EntityLocation {
                            archetype_id: swapped_location.archetype_id,
                            archetype_row: location.archetype_row,
                            table_id: swapped_location.table_id,
                            table_row: swapped_location.table_row,
                        },
                    );
                }
                // PERF: store "non bundle" components in edge, then just move those to avoid
                // redundant copies
                let move_result = table.move_to_superset_unchecked(result.table_row, new_table);
                let new_location = new_archetype.allocate(entity, move_result.new_row);
                world.entities.set(entity.index(), new_location);

                // if an entity was moved into this entity's table spot, update its table row
                if let Some(swapped_entity) = move_result.swapped_entity {
                    let swapped_location =
                        // SAFETY: If the swap was successful, swapped_entity must be valid.
                        unsafe { world.entities.get(swapped_entity).debug_checked_unwrap() };
                    let swapped_archetype = if archetype.id() == swapped_location.archetype_id {
                        &mut *archetype
                    } else if new_archetype.id() == swapped_location.archetype_id {
                        new_archetype
                    } else {
                        // SAFETY: the only two borrowed archetypes are above and we just did collision checks
                        &mut world.archetypes[swapped_location.archetype_id]
                    };

                    world.entities.set(
                        swapped_entity.index(),
                        EntityLocation {
                            archetype_id: swapped_location.archetype_id,
                            archetype_row: swapped_location.archetype_row,
                            table_id: swapped_location.table_id,
                            table_row: result.table_row,
                        },
                    );
                    swapped_archetype
                        .set_entity_table_row(swapped_location.archetype_row, result.table_row);
                }

                bundle_info.write_components(
                    new_table,
                    &mut world.storages.sparse_sets,
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

    #[inline]
    pub(crate) fn entities(&mut self) -> &mut Entities {
        &mut self.world.entities
    }
}

pub(crate) struct BundleSpawner<'w> {
    world: &'w mut World,
    bundle_info: *const BundleInfo,
    archetype: *mut Archetype,
    table: *mut Table,
    change_tick: Tick,
}

impl<'w> BundleSpawner<'w> {
    #[inline]
    pub fn new<T: Bundle>(world: &'w mut World, change_tick: Tick) -> Self {
        let bundle_info: *const BundleInfo = world
            .bundles
            .init_info::<T>(&mut world.components, &mut world.storages);
        unsafe { Self::new_with_info(world, bundle_info, change_tick) }
    }

    pub(crate) unsafe fn new_with_info(
        world: &'w mut World,
        bundle_info: *const BundleInfo,
        change_tick: Tick,
    ) -> Self {
        let bundle_info = &*bundle_info;
        let new_archetype_id = bundle_info.add_bundle_to_archetype(
            &mut world.archetypes,
            &mut world.storages,
            &world.components,
            ArchetypeId::EMPTY,
        );
        let archetype = &mut world.archetypes[new_archetype_id];
        let table: *mut Table = &mut world.storages.tables[archetype.table_id()];
        let archetype: *mut Archetype = archetype;
        BundleSpawner {
            world,
            bundle_info,
            archetype,
            table,
            change_tick,
        }
    }

    pub fn reserve_storage(&mut self, additional: usize) {
        unsafe {
            (&mut *self.archetype).reserve(additional);
            (&mut *self.table).reserve(additional);
        }
    }
    /// # Safety
    /// `entity` must be allocated (but non-existent), `T` must match this [`BundleInfo`]'s type
    #[inline]
    pub unsafe fn spawn_non_existent<T: DynamicBundle>(
        &mut self,
        entity: Entity,
        bundle: T,
    ) -> EntityLocation {
        let bundle_info = &*self.bundle_info;
        for component_id in bundle_info.components().iter().cloned() {
            let hooks = self
                .world
                .components
                .get_info_unchecked(component_id)
                .hooks();
            if let Some(hook) = hooks.on_add {
                hook(self.world.into_deferred(), entity);
            }
            if let Some(hook) = hooks.on_insert {
                hook(self.world.into_deferred(), entity);
            }
        }

        let archetype = &mut *self.archetype;
        let table = &mut *self.table;

        let table_row = table.allocate(entity);
        let location = archetype.allocate(entity, table_row);
        bundle_info.write_components(
            table,
            &mut self.world.storages.sparse_sets,
            &SpawnBundleStatus,
            entity,
            table_row,
            self.change_tick,
            bundle,
        );
        self.world.entities.set(entity.index(), location);

        location
    }

    /// # Safety
    /// `T` must match this [`BundleInfo`]'s type
    #[inline]
    pub unsafe fn spawn<T: Bundle>(&mut self, bundle: T) -> Entity {
        let entity = self.world.entities.alloc();
        // SAFETY: entity is allocated (but non-existent), `T` matches this BundleInfo's type
        self.spawn_non_existent(entity, bundle);
        entity
    }

    #[inline]
    pub(crate) fn entities(&mut self) -> &mut Entities {
        &mut self.world.entities
    }
}

/// Metadata for bundles. Stores a [`BundleInfo`] for each type of [`Bundle`] in a given world.
#[derive(Default)]
pub struct Bundles {
    bundle_infos: Vec<BundleInfo>,
    /// Cache static [`BundleId`]
    bundle_ids: TypeIdMap<BundleId>,
    /// Cache dynamic [`BundleId`] with multiple components
    dynamic_bundle_ids: HashMap<Vec<ComponentId>, (BundleId, Vec<StorageType>)>,
    /// Cache optimized dynamic [`BundleId`] with single component
    dynamic_component_bundle_ids: HashMap<ComponentId, (BundleId, StorageType)>,
}

impl Bundles {
    /// Gets the metadata associated with a specific type of bundle.
    /// Returns `None` if the bundle is not registered with the world.
    #[inline]
    pub fn get(&self, bundle_id: BundleId) -> Option<&BundleInfo> {
        self.bundle_infos.get(bundle_id.index())
    }

    /// Gets the value identifying a specific type of bundle.
    /// Returns `None` if the bundle does not exist in the world,
    /// or if `type_id` does not correspond to a type of bundle.
    #[inline]
    pub fn get_id(&self, type_id: TypeId) -> Option<BundleId> {
        self.bundle_ids.get(&type_id).cloned()
    }

    /// Initializes a new [`BundleInfo`] for a statically known type.
    pub(crate) fn init_info<'a, T: Bundle>(
        &'a mut self,
        components: &mut Components,
        storages: &mut Storages,
    ) -> &'a BundleInfo {
        let bundle_infos = &mut self.bundle_infos;
        let id = *self.bundle_ids.entry(TypeId::of::<T>()).or_insert_with(|| {
            let mut component_ids = Vec::new();
            T::component_ids(components, storages, &mut |id| component_ids.push(id));
            let id = BundleId(bundle_infos.len());
            let bundle_info =
                // SAFETY: T::component_id ensures its:
                // - info was created
                // - appropriate storage for it has been initialized.
                // - was created in the same order as the components in T
                unsafe { BundleInfo::new(std::any::type_name::<T>(), components, component_ids, id) };
            bundle_infos.push(bundle_info);
            id
        });
        // SAFETY: index either exists, or was initialized
        unsafe { self.get_unchecked(id) }
    }

    pub(crate) unsafe fn get_unchecked<'a>(&'a self, id: BundleId) -> &'a BundleInfo {
        self.bundle_infos.get_unchecked(id.0)
    }

    /// Initializes a new [`BundleInfo`] for a dynamic [`Bundle`].
    ///
    /// # Panics
    ///
    /// Panics if any of the provided [`ComponentId`]s do not exist in the
    /// provided [`Components`].
    pub(crate) fn init_dynamic_info(
        &mut self,
        components: &Components,
        component_ids: &[ComponentId],
    ) -> (&BundleInfo, &Vec<StorageType>) {
        let bundle_infos = &mut self.bundle_infos;

        // Use `raw_entry_mut` to avoid cloning `component_ids` to access `Entry`
        let (_, (bundle_id, storage_types)) = self
            .dynamic_bundle_ids
            .raw_entry_mut()
            .from_key(component_ids)
            .or_insert_with(|| {
                (
                    Vec::from(component_ids),
                    initialize_dynamic_bundle(bundle_infos, components, Vec::from(component_ids)),
                )
            });

        // SAFETY: index either exists, or was initialized
        let bundle_info = unsafe { bundle_infos.get_unchecked(bundle_id.0) };

        (bundle_info, storage_types)
    }

    /// Initializes a new [`BundleInfo`] for a dynamic [`Bundle`] with single component.
    ///
    /// # Panics
    ///
    /// Panics if the provided [`ComponentId`] does not exist in the provided [`Components`].
    pub(crate) fn init_component_info(
        &mut self,
        components: &Components,
        component_id: ComponentId,
    ) -> (&BundleInfo, StorageType) {
        let bundle_infos = &mut self.bundle_infos;
        let (bundle_id, storage_types) = self
            .dynamic_component_bundle_ids
            .entry(component_id)
            .or_insert_with(|| {
                let (id, storage_type) =
                    initialize_dynamic_bundle(bundle_infos, components, vec![component_id]);
                // SAFETY: `storage_type` guaranteed to have length 1
                (id, storage_type[0])
            });

        // SAFETY: index either exists, or was initialized
        let bundle_info = unsafe { bundle_infos.get_unchecked(bundle_id.0) };

        (bundle_info, *storage_types)
    }
}

/// Asserts that all components are part of [`Components`]
/// and initializes a [`BundleInfo`].
fn initialize_dynamic_bundle(
    bundle_infos: &mut Vec<BundleInfo>,
    components: &Components,
    component_ids: Vec<ComponentId>,
) -> (BundleId, Vec<StorageType>) {
    // Assert component existence
    let storage_types = component_ids.iter().map(|&id| {
        components.get_info(id).unwrap_or_else(|| {
            panic!(
                "init_dynamic_info called with component id {id:?} which doesn't exist in this world"
            )
        }).storage_type()
    }).collect();

    let id = BundleId(bundle_infos.len());
    let bundle_info =
        // SAFETY: `component_ids` are valid as they were just checked
        unsafe { BundleInfo::new("<dynamic bundle>", components, component_ids, id) };
    bundle_infos.push(bundle_info);

    (id, storage_types)
}

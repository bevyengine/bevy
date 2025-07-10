//! Types for handling [`Bundle`]s.
//!
//! This module contains the [`Bundle`] trait and some other helper types.

mod impls;
mod insert;
#[cfg(test)]
mod tests;

/// Derive the [`Bundle`] trait
///
/// You can apply this derive macro to structs that are
/// composed of [`Component`]s or
/// other [`Bundle`]s.
///
/// ## Attributes
///
/// Sometimes parts of the Bundle should not be inserted.
/// Those can be marked with `#[bundle(ignore)]`, and they will be skipped.
/// In that case, the field needs to implement [`Default`] unless you also ignore
/// the [`BundleFromComponents`] implementation.
///
/// ```rust
/// # use bevy_ecs::prelude::{Component, Bundle};
/// # #[derive(Component)]
/// # struct Hitpoint;
/// #
/// #[derive(Bundle)]
/// struct HitpointMarker {
///     hitpoints: Hitpoint,
///
///     #[bundle(ignore)]
///     creator: Option<String>
/// }
/// ```
///
/// Some fields may be bundles that do not implement
/// [`BundleFromComponents`]. This happens for bundles that cannot be extracted.
/// For example with [`SpawnRelatedBundle`](bevy_ecs::spawn::SpawnRelatedBundle), see below for an
/// example usage.
/// In those cases you can either ignore it as above,
/// or you can opt out the whole Struct by marking it as ignored with
/// `#[bundle(ignore_from_components)]`.
///
/// ```rust
/// # use bevy_ecs::prelude::{Component, Bundle, ChildOf, Spawn};
/// # #[derive(Component)]
/// # struct Hitpoint;
/// # #[derive(Component)]
/// # struct Marker;
/// #
/// use bevy_ecs::spawn::SpawnRelatedBundle;
///
/// #[derive(Bundle)]
/// #[bundle(ignore_from_components)]
/// struct HitpointMarker {
///     hitpoints: Hitpoint,
///     related_spawner: SpawnRelatedBundle<ChildOf, Spawn<Marker>>,
/// }
/// ```
pub use bevy_ecs_macros::Bundle;

use crate::{
    archetype::{
        Archetype, ArchetypeCreated, ArchetypeId, Archetypes, BundleComponentStatus,
        ComponentStatus, SpawnBundleStatus,
    },
    change_detection::MaybeLocation,
    component::{
        ComponentId, Components, ComponentsRegistrator, RequiredComponentConstructor,
        RequiredComponents, StorageType, Tick,
    },
    entity::{Entities, Entity, EntityLocation},
    lifecycle::{ADD, INSERT, REMOVE, REPLACE},
    observer::Observers,
    prelude::World,
    query::DebugCheckedUnwrap,
    relationship::RelationshipHookMode,
    storage::{SparseSetIndex, SparseSets, Storages, Table, TableRow},
    world::{unsafe_world_cell::UnsafeWorldCell, EntityWorldMut},
};
use alloc::{boxed::Box, vec, vec::Vec};
use bevy_platform::collections::{HashMap, HashSet};
use bevy_ptr::{ConstNonNull, OwningPtr};
use bevy_utils::TypeIdMap;
use core::{any::TypeId, ptr::NonNull};

/// The `Bundle` trait enables insertion and removal of [`Component`]s from an entity.
///
/// Implementers of the `Bundle` trait are called 'bundles'.
///
/// Each bundle represents a static set of [`Component`] types.
/// Currently, bundles can only contain one of each [`Component`], and will
/// panic once initialized if this is not met.
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
/// # Implementers
///
/// Every type which implements [`Component`] also implements `Bundle`, since
/// [`Component`] types can be added to or removed from an entity.
///
/// Additionally, [Tuples](`tuple`) of bundles are also [`Bundle`] (with up to 15 bundles).
/// These bundles contain the items of the 'inner' bundles.
/// This is a convenient shorthand which is primarily used when spawning entities.
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
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a `Bundle`",
    label = "invalid `Bundle`",
    note = "consider annotating `{Self}` with `#[derive(Component)]` or `#[derive(Bundle)]`"
)]
pub unsafe trait Bundle: DynamicBundle + Send + Sync + 'static {
    /// Gets this [`Bundle`]'s component ids, in the order of this bundle's [`Component`]s
    #[doc(hidden)]
    fn component_ids(components: &mut ComponentsRegistrator, ids: &mut impl FnMut(ComponentId));

    /// Gets this [`Bundle`]'s component ids. This will be [`None`] if the component has not been registered.
    fn get_component_ids(components: &Components, ids: &mut impl FnMut(Option<ComponentId>));
}

/// Creates a [`Bundle`] by taking it from internal storage.
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
pub unsafe trait BundleFromComponents {
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
    /// An operation on the entity that happens _after_ inserting this bundle.
    type Effect: BundleEffect;
    // SAFETY:
    // The `StorageType` argument passed into [`Bundle::get_components`] must be correct for the
    // component being fetched.
    //
    /// Calls `func` on each value, in the order of this bundle's [`Component`]s. This passes
    /// ownership of the component values to `func`.
    #[doc(hidden)]
    fn get_components(self, func: &mut impl FnMut(StorageType, OwningPtr<'_>)) -> Self::Effect;
}

/// An operation on an [`Entity`] that occurs _after_ inserting the [`Bundle`] that defined this bundle effect.
/// The order of operations is:
///
/// 1. The [`Bundle`] is inserted on the entity
/// 2. Relevant Hooks are run for the insert, then Observers
/// 3. The [`BundleEffect`] is run.
///
/// See [`DynamicBundle::Effect`].
pub trait BundleEffect {
    /// Applies this effect to the given `entity`.
    fn apply(self, entity: &mut EntityWorldMut);
}

/// A trait implemented for [`BundleEffect`] implementations that do nothing. This is used as a type constraint for
/// [`Bundle`] APIs that do not / cannot run [`DynamicBundle::Effect`], such as "batch spawn" APIs.
pub trait NoBundleEffect {}

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

/// What to do on insertion if a component already exists.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum InsertMode {
    /// Any existing components of a matching type will be overwritten.
    Replace,
    /// Any existing components of a matching type will be left unchanged.
    Keep,
}

/// Stores metadata associated with a specific type of [`Bundle`] for a given [`World`].
///
/// [`World`]: crate::world::World
pub struct BundleInfo {
    id: BundleId,
    /// The list of all components contributed by the bundle (including Required Components). This is in
    /// the order `[EXPLICIT_COMPONENTS][REQUIRED_COMPONENTS]`
    ///
    /// # Safety
    /// Every ID in this list must be valid within the World that owns the [`BundleInfo`],
    /// must have its storage initialized (i.e. columns created in tables, sparse set created),
    /// and the range (0..`explicit_components_len`) must be in the same order as the source bundle
    /// type writes its components in.
    component_ids: Vec<ComponentId>,
    required_components: Vec<RequiredComponentConstructor>,
    explicit_components_len: usize,
}

impl BundleInfo {
    /// Create a new [`BundleInfo`].
    ///
    /// # Safety
    ///
    /// Every ID in `component_ids` must be valid within the World that owns the `BundleInfo`
    /// and must be in the same order as the source bundle type writes its components in.
    unsafe fn new(
        bundle_type_name: &'static str,
        storages: &mut Storages,
        components: &Components,
        mut component_ids: Vec<ComponentId>,
        id: BundleId,
    ) -> BundleInfo {
        // check for duplicates
        let mut deduped = component_ids.clone();
        deduped.sort_unstable();
        deduped.dedup();
        if deduped.len() != component_ids.len() {
            // TODO: Replace with `Vec::partition_dedup` once https://github.com/rust-lang/rust/issues/54279 is stabilized
            let mut seen = <HashSet<_>>::default();
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
                .collect::<Vec<_>>();

            panic!("Bundle {bundle_type_name} has duplicate components: {names:?}");
        }

        // handle explicit components
        let explicit_components_len = component_ids.len();
        let mut required_components = RequiredComponents::default();
        for component_id in component_ids.iter().copied() {
            // SAFETY: caller has verified that all ids are valid
            let info = unsafe { components.get_info_unchecked(component_id) };
            required_components.merge(info.required_components());
            storages.prepare_component(info);
        }
        required_components.remove_explicit_components(&component_ids);

        // handle required components
        let required_components = required_components
            .0
            .into_iter()
            .map(|(component_id, v)| {
                // Safety: These ids came out of the passed `components`, so they must be valid.
                let info = unsafe { components.get_info_unchecked(component_id) };
                storages.prepare_component(info);
                // This adds required components to the component_ids list _after_ using that list to remove explicitly provided
                // components. This ordering is important!
                component_ids.push(component_id);
                v.constructor
            })
            .collect();

        // SAFETY: The caller ensures that component_ids:
        // - is valid for the associated world
        // - has had its storage initialized
        // - is in the same order as the source bundle type
        BundleInfo {
            id,
            component_ids,
            required_components,
            explicit_components_len,
        }
    }

    /// Returns a value identifying the associated [`Bundle`] type.
    #[inline]
    pub const fn id(&self) -> BundleId {
        self.id
    }

    /// Returns the [ID](ComponentId) of each component explicitly defined in this bundle (ex: Required Components are excluded).
    ///
    /// For all components contributed by this bundle (including Required Components), see [`BundleInfo::contributed_components`]
    #[inline]
    pub fn explicit_components(&self) -> &[ComponentId] {
        &self.component_ids[0..self.explicit_components_len]
    }

    /// Returns the [ID](ComponentId) of each Required Component needed by this bundle. This _does not include_ Required Components that are
    /// explicitly provided by the bundle.
    #[inline]
    pub fn required_components(&self) -> &[ComponentId] {
        &self.component_ids[self.explicit_components_len..]
    }

    /// Returns the [ID](ComponentId) of each component contributed by this bundle. This includes Required Components.
    ///
    /// For only components explicitly defined in this bundle, see [`BundleInfo::explicit_components`]
    #[inline]
    pub fn contributed_components(&self) -> &[ComponentId] {
        &self.component_ids
    }

    /// Returns an iterator over the [ID](ComponentId) of each component explicitly defined in this bundle (ex: this excludes Required Components).
    /// To iterate all components contributed by this bundle (including Required Components), see [`BundleInfo::iter_contributed_components`]
    #[inline]
    pub fn iter_explicit_components(&self) -> impl Iterator<Item = ComponentId> + Clone + '_ {
        self.explicit_components().iter().copied()
    }

    /// Returns an iterator over the [ID](ComponentId) of each component contributed by this bundle. This includes Required Components.
    ///
    /// To iterate only components explicitly defined in this bundle, see [`BundleInfo::iter_explicit_components`]
    #[inline]
    pub fn iter_contributed_components(&self) -> impl Iterator<Item = ComponentId> + Clone + '_ {
        self.component_ids.iter().copied()
    }

    /// Returns an iterator over the [ID](ComponentId) of each Required Component needed by this bundle. This _does not include_ Required Components that are
    /// explicitly provided by the bundle.
    pub fn iter_required_components(&self) -> impl Iterator<Item = ComponentId> + '_ {
        self.required_components().iter().copied()
    }

    /// This writes components from a given [`Bundle`] to the given entity.
    ///
    /// # Safety
    ///
    /// `bundle_component_status` must return the "correct" [`ComponentStatus`] for each component
    /// in the [`Bundle`], with respect to the entity's original archetype (prior to the bundle being added).
    ///
    /// For example, if the original archetype already has `ComponentA` and `T` also has `ComponentA`, the status
    /// should be `Existing`. If the original archetype does not have `ComponentA`, the status should be `Added`.
    ///
    /// When "inserting" a bundle into an existing entity, [`ArchetypeAfterBundleInsert`]
    /// should be used, which will report `Added` vs `Existing` status based on the current archetype's structure.
    ///
    /// When spawning a bundle, [`SpawnBundleStatus`] can be used instead, which removes the need
    /// to look up the [`ArchetypeAfterBundleInsert`] in the archetype graph, which requires
    /// ownership of the entity's current archetype.
    ///
    /// `table` must be the "new" table for `entity`. `table_row` must have space allocated for the
    /// `entity`, `bundle` must match this [`BundleInfo`]'s type
    #[inline]
    unsafe fn write_components<'a, T: DynamicBundle, S: BundleComponentStatus>(
        &self,
        table: &mut Table,
        sparse_sets: &mut SparseSets,
        bundle_component_status: &S,
        required_components: impl Iterator<Item = &'a RequiredComponentConstructor>,
        entity: Entity,
        table_row: TableRow,
        change_tick: Tick,
        bundle: T,
        insert_mode: InsertMode,
        caller: MaybeLocation,
    ) -> T::Effect {
        // NOTE: get_components calls this closure on each component in "bundle order".
        // bundle_info.component_ids are also in "bundle order"
        let mut bundle_component = 0;
        let after_effect = bundle.get_components(&mut |storage_type, component_ptr| {
            let component_id = *self.component_ids.get_unchecked(bundle_component);
            // SAFETY: bundle_component is a valid index for this bundle
            let status = unsafe { bundle_component_status.get_status(bundle_component) };
            match storage_type {
                StorageType::Table => {
                    let column =
                        // SAFETY: If component_id is in self.component_ids, BundleInfo::new ensures that
                        // the target table contains the component.
                        unsafe { table.get_column_mut(component_id).debug_checked_unwrap() };
                    match (status, insert_mode) {
                        (ComponentStatus::Added, _) => {
                            column.initialize(table_row, component_ptr, change_tick, caller);
                        }
                        (ComponentStatus::Existing, InsertMode::Replace) => {
                            column.replace(table_row, component_ptr, change_tick, caller);
                        }
                        (ComponentStatus::Existing, InsertMode::Keep) => {
                            if let Some(drop_fn) = table.get_drop_for(component_id) {
                                drop_fn(component_ptr);
                            }
                        }
                    }
                }
                StorageType::SparseSet => {
                    let sparse_set =
                        // SAFETY: If component_id is in self.component_ids, BundleInfo::new ensures that
                        // a sparse set exists for the component.
                        unsafe { sparse_sets.get_mut(component_id).debug_checked_unwrap() };
                    match (status, insert_mode) {
                        (ComponentStatus::Added, _) | (_, InsertMode::Replace) => {
                            sparse_set.insert(entity, component_ptr, change_tick, caller);
                        }
                        (ComponentStatus::Existing, InsertMode::Keep) => {
                            if let Some(drop_fn) = sparse_set.get_drop() {
                                drop_fn(component_ptr);
                            }
                        }
                    }
                }
            }
            bundle_component += 1;
        });

        for required_component in required_components {
            required_component.initialize(
                table,
                sparse_sets,
                change_tick,
                table_row,
                entity,
                caller,
            );
        }

        after_effect
    }

    /// Internal method to initialize a required component from an [`OwningPtr`]. This should ultimately be called
    /// in the context of [`BundleInfo::write_components`], via [`RequiredComponentConstructor::initialize`].
    ///
    /// # Safety
    ///
    /// `component_ptr` must point to a required component value that matches the given `component_id`. The `storage_type` must match
    /// the type associated with `component_id`. The `entity` and `table_row` must correspond to an entity with an uninitialized
    /// component matching `component_id`.
    ///
    /// This method _should not_ be called outside of [`BundleInfo::write_components`].
    /// For more information, read the [`BundleInfo::write_components`] safety docs.
    /// This function inherits the safety requirements defined there.
    pub(crate) unsafe fn initialize_required_component(
        table: &mut Table,
        sparse_sets: &mut SparseSets,
        change_tick: Tick,
        table_row: TableRow,
        entity: Entity,
        component_id: ComponentId,
        storage_type: StorageType,
        component_ptr: OwningPtr,
        caller: MaybeLocation,
    ) {
        {
            match storage_type {
                StorageType::Table => {
                    let column =
                        // SAFETY: If component_id is in required_components, BundleInfo::new requires that
                        // the target table contains the component.
                        unsafe { table.get_column_mut(component_id).debug_checked_unwrap() };
                    column.initialize(table_row, component_ptr, change_tick, caller);
                }
                StorageType::SparseSet => {
                    let sparse_set =
                        // SAFETY: If component_id is in required_components, BundleInfo::new requires that
                        // a sparse set exists for the component.
                        unsafe { sparse_sets.get_mut(component_id).debug_checked_unwrap() };
                    sparse_set.insert(entity, component_ptr, change_tick, caller);
                }
            }
        }
    }

    /// Removes a bundle from the given archetype and returns the resulting archetype and whether a new archetype was created.
    /// (or `None` if the removal was invalid).
    /// This could be the same [`ArchetypeId`], in the event that removing the given bundle
    /// does not result in an [`Archetype`] change.
    ///
    /// Results are cached in the [`Archetype`] graph to avoid redundant work.
    ///
    /// If `intersection` is false, attempting to remove a bundle with components not contained in the
    /// current archetype will fail, returning `None`.
    ///
    /// If `intersection` is true, components in the bundle but not in the current archetype
    /// will be ignored.
    ///
    /// # Safety
    /// `archetype_id` must exist and components in `bundle_info` must exist
    pub(crate) unsafe fn remove_bundle_from_archetype(
        &self,
        archetypes: &mut Archetypes,
        storages: &mut Storages,
        components: &Components,
        observers: &Observers,
        archetype_id: ArchetypeId,
        intersection: bool,
    ) -> (Option<ArchetypeId>, bool) {
        // Check the archetype graph to see if the bundle has been
        // removed from this archetype in the past.
        let archetype_after_remove_result = {
            let edges = archetypes[archetype_id].edges();
            if intersection {
                edges.get_archetype_after_bundle_remove(self.id())
            } else {
                edges.get_archetype_after_bundle_take(self.id())
            }
        };
        let (result, is_new_created) = if let Some(result) = archetype_after_remove_result {
            // This bundle removal result is cached. Just return that!
            (result, false)
        } else {
            let mut next_table_components;
            let mut next_sparse_set_components;
            let next_table_id;
            {
                let current_archetype = &mut archetypes[archetype_id];
                let mut removed_table_components = Vec::new();
                let mut removed_sparse_set_components = Vec::new();
                for component_id in self.iter_explicit_components() {
                    if current_archetype.contains(component_id) {
                        // SAFETY: bundle components were already initialized by bundles.get_info
                        let component_info = unsafe { components.get_info_unchecked(component_id) };
                        match component_info.storage_type() {
                            StorageType::Table => removed_table_components.push(component_id),
                            StorageType::SparseSet => {
                                removed_sparse_set_components.push(component_id);
                            }
                        }
                    } else if !intersection {
                        // A component in the bundle was not present in the entity's archetype, so this
                        // removal is invalid. Cache the result in the archetype graph.
                        current_archetype
                            .edges_mut()
                            .cache_archetype_after_bundle_take(self.id(), None);
                        return (None, false);
                    }
                }

                // Sort removed components so we can do an efficient "sorted remove".
                // Archetype components are already sorted.
                removed_table_components.sort_unstable();
                removed_sparse_set_components.sort_unstable();
                next_table_components = current_archetype.table_components().collect();
                next_sparse_set_components = current_archetype.sparse_set_components().collect();
                sorted_remove(&mut next_table_components, &removed_table_components);
                sorted_remove(
                    &mut next_sparse_set_components,
                    &removed_sparse_set_components,
                );

                next_table_id = if removed_table_components.is_empty() {
                    current_archetype.table_id()
                } else {
                    // SAFETY: all components in next_table_components exist
                    unsafe {
                        storages
                            .tables
                            .get_id_or_insert(&next_table_components, components)
                    }
                };
            }

            let (new_archetype_id, is_new_created) = archetypes.get_id_or_insert(
                components,
                observers,
                next_table_id,
                next_table_components,
                next_sparse_set_components,
            );
            (Some(new_archetype_id), is_new_created)
        };
        let current_archetype = &mut archetypes[archetype_id];
        // Cache the result in an edge.
        if intersection {
            current_archetype
                .edges_mut()
                .cache_archetype_after_bundle_remove(self.id(), result);
        } else {
            current_archetype
                .edges_mut()
                .cache_archetype_after_bundle_take(self.id(), result);
        }
        (result, is_new_created)
    }
}

/// The type of archetype move (or lack thereof) that will result from a bundle
/// being inserted into an entity.
pub(crate) enum ArchetypeMoveType {
    /// If the entity already has all of the components that are being inserted,
    /// its archetype won't change.
    SameArchetype,
    /// If only [`sparse set`](StorageType::SparseSet) components are being added,
    /// the entity's archetype will change while keeping the same table.
    NewArchetypeSameTable { new_archetype: NonNull<Archetype> },
    /// If any [`table-stored`](StorageType::Table) components are being added,
    /// both the entity's archetype and table will change.
    NewArchetypeNewTable {
        new_archetype: NonNull<Archetype>,
        new_table: NonNull<Table>,
    },
}

// SAFETY: We have exclusive world access so our pointers can't be invalidated externally
pub(crate) struct BundleRemover<'w> {
    world: UnsafeWorldCell<'w>,
    bundle_info: ConstNonNull<BundleInfo>,
    old_and_new_table: Option<(NonNull<Table>, NonNull<Table>)>,
    old_archetype: NonNull<Archetype>,
    new_archetype: NonNull<Archetype>,
}

impl<'w> BundleRemover<'w> {
    /// Creates a new [`BundleRemover`], if such a remover would do anything.
    ///
    /// If `require_all` is true, the [`BundleRemover`] is only created if the entire bundle is present on the archetype.
    ///
    /// # Safety
    /// Caller must ensure that `archetype_id` is valid
    #[inline]
    pub(crate) unsafe fn new<T: Bundle>(
        world: &'w mut World,
        archetype_id: ArchetypeId,
        require_all: bool,
    ) -> Option<Self> {
        // SAFETY: These come from the same world. `world.components_registrator` can't be used since we borrow other fields too.
        let mut registrator =
            unsafe { ComponentsRegistrator::new(&mut world.components, &mut world.component_ids) };
        let bundle_id = world
            .bundles
            .register_info::<T>(&mut registrator, &mut world.storages);
        // SAFETY: we initialized this bundle_id in `init_info`, and caller ensures archetype is valid.
        unsafe { Self::new_with_id(world, archetype_id, bundle_id, require_all) }
    }

    /// Creates a new [`BundleRemover`], if such a remover would do anything.
    ///
    /// If `require_all` is true, the [`BundleRemover`] is only created if the entire bundle is present on the archetype.
    ///
    /// # Safety
    /// Caller must ensure that `bundle_id` exists in `world.bundles` and `archetype_id` is valid.
    #[inline]
    pub(crate) unsafe fn new_with_id(
        world: &'w mut World,
        archetype_id: ArchetypeId,
        bundle_id: BundleId,
        require_all: bool,
    ) -> Option<Self> {
        let bundle_info = world.bundles.get_unchecked(bundle_id);
        // SAFETY: Caller ensures archetype and bundle ids are correct.
        let (new_archetype_id, is_new_created) = unsafe {
            bundle_info.remove_bundle_from_archetype(
                &mut world.archetypes,
                &mut world.storages,
                &world.components,
                &world.observers,
                archetype_id,
                !require_all,
            )
        };
        let new_archetype_id = new_archetype_id?;

        if new_archetype_id == archetype_id {
            return None;
        }

        let (old_archetype, new_archetype) =
            world.archetypes.get_2_mut(archetype_id, new_archetype_id);

        let tables = if old_archetype.table_id() == new_archetype.table_id() {
            None
        } else {
            let (old, new) = world
                .storages
                .tables
                .get_2_mut(old_archetype.table_id(), new_archetype.table_id());
            Some((old.into(), new.into()))
        };

        let remover = Self {
            bundle_info: bundle_info.into(),
            new_archetype: new_archetype.into(),
            old_archetype: old_archetype.into(),
            old_and_new_table: tables,
            world: world.as_unsafe_world_cell(),
        };
        if is_new_created {
            remover
                .world
                .into_deferred()
                .trigger(ArchetypeCreated(new_archetype_id));
        }
        Some(remover)
    }

    /// This can be passed to [`remove`](Self::remove) as the `pre_remove` function if you don't want to do anything before removing.
    pub fn empty_pre_remove(
        _: &mut SparseSets,
        _: Option<&mut Table>,
        _: &Components,
        _: &[ComponentId],
    ) -> (bool, ()) {
        (true, ())
    }

    /// Performs the removal.
    ///
    /// `pre_remove` should return a bool for if the components still need to be dropped.
    ///
    /// # Safety
    /// The `location` must have the same archetype as the remover.
    #[inline]
    pub(crate) unsafe fn remove<T: 'static>(
        &mut self,
        entity: Entity,
        location: EntityLocation,
        caller: MaybeLocation,
        pre_remove: impl FnOnce(
            &mut SparseSets,
            Option<&mut Table>,
            &Components,
            &[ComponentId],
        ) -> (bool, T),
    ) -> (EntityLocation, T) {
        // Hooks
        // SAFETY: all bundle components exist in World
        unsafe {
            // SAFETY: We only keep access to archetype/bundle data.
            let mut deferred_world = self.world.into_deferred();
            let bundle_components_in_archetype = || {
                self.bundle_info
                    .as_ref()
                    .iter_explicit_components()
                    .filter(|component_id| self.old_archetype.as_ref().contains(*component_id))
            };
            if self.old_archetype.as_ref().has_replace_observer() {
                deferred_world.trigger_observers(
                    REPLACE,
                    Some(entity),
                    bundle_components_in_archetype(),
                    caller,
                );
            }
            deferred_world.trigger_on_replace(
                self.old_archetype.as_ref(),
                entity,
                bundle_components_in_archetype(),
                caller,
                RelationshipHookMode::Run,
            );
            if self.old_archetype.as_ref().has_remove_observer() {
                deferred_world.trigger_observers(
                    REMOVE,
                    Some(entity),
                    bundle_components_in_archetype(),
                    caller,
                );
            }
            deferred_world.trigger_on_remove(
                self.old_archetype.as_ref(),
                entity,
                bundle_components_in_archetype(),
                caller,
            );
        }

        // SAFETY: We still have the cell, so this is unique, it doesn't conflict with other references, and we drop it shortly.
        let world = unsafe { self.world.world_mut() };

        let (needs_drop, pre_remove_result) = pre_remove(
            &mut world.storages.sparse_sets,
            self.old_and_new_table
                .as_ref()
                // SAFETY: There is no conflicting access for this scope.
                .map(|(old, _)| unsafe { &mut *old.as_ptr() }),
            &world.components,
            self.bundle_info.as_ref().explicit_components(),
        );

        // Handle sparse set removes
        for component_id in self.bundle_info.as_ref().iter_explicit_components() {
            if self.old_archetype.as_ref().contains(component_id) {
                world.removed_components.write(component_id, entity);

                // Make sure to drop components stored in sparse sets.
                // Dense components are dropped later in `move_to_and_drop_missing_unchecked`.
                if let Some(StorageType::SparseSet) =
                    self.old_archetype.as_ref().get_storage_type(component_id)
                {
                    world
                        .storages
                        .sparse_sets
                        .get_mut(component_id)
                        // Set exists because the component existed on the entity
                        .unwrap()
                        // If it was already forgotten, it would not be in the set.
                        .remove(entity);
                }
            }
        }

        // Handle archetype change
        let remove_result = self
            .old_archetype
            .as_mut()
            .swap_remove(location.archetype_row);
        // if an entity was moved into this entity's archetype row, update its archetype row
        if let Some(swapped_entity) = remove_result.swapped_entity {
            let swapped_location = world.entities.get(swapped_entity).unwrap();

            world.entities.set(
                swapped_entity.index(),
                Some(EntityLocation {
                    archetype_id: swapped_location.archetype_id,
                    archetype_row: location.archetype_row,
                    table_id: swapped_location.table_id,
                    table_row: swapped_location.table_row,
                }),
            );
        }

        // Handle table change
        let new_location = if let Some((mut old_table, mut new_table)) = self.old_and_new_table {
            let move_result = if needs_drop {
                // SAFETY: old_table_row exists
                unsafe {
                    old_table
                        .as_mut()
                        .move_to_and_drop_missing_unchecked(location.table_row, new_table.as_mut())
                }
            } else {
                // SAFETY: old_table_row exists
                unsafe {
                    old_table.as_mut().move_to_and_forget_missing_unchecked(
                        location.table_row,
                        new_table.as_mut(),
                    )
                }
            };

            // SAFETY: move_result.new_row is a valid position in new_archetype's table
            let new_location = unsafe {
                self.new_archetype
                    .as_mut()
                    .allocate(entity, move_result.new_row)
            };

            // if an entity was moved into this entity's table row, update its table row
            if let Some(swapped_entity) = move_result.swapped_entity {
                let swapped_location = world.entities.get(swapped_entity).unwrap();

                world.entities.set(
                    swapped_entity.index(),
                    Some(EntityLocation {
                        archetype_id: swapped_location.archetype_id,
                        archetype_row: swapped_location.archetype_row,
                        table_id: swapped_location.table_id,
                        table_row: location.table_row,
                    }),
                );
                world.archetypes[swapped_location.archetype_id]
                    .set_entity_table_row(swapped_location.archetype_row, location.table_row);
            }

            new_location
        } else {
            // The tables are the same
            self.new_archetype
                .as_mut()
                .allocate(entity, location.table_row)
        };

        // SAFETY: The entity is valid and has been moved to the new location already.
        unsafe {
            world.entities.set(entity.index(), Some(new_location));
        }

        (new_location, pre_remove_result)
    }
}

// SAFETY: We have exclusive world access so our pointers can't be invalidated externally
pub(crate) struct BundleSpawner<'w> {
    world: UnsafeWorldCell<'w>,
    bundle_info: ConstNonNull<BundleInfo>,
    table: NonNull<Table>,
    archetype: NonNull<Archetype>,
    change_tick: Tick,
}

impl<'w> BundleSpawner<'w> {
    #[inline]
    pub fn new<T: Bundle>(world: &'w mut World, change_tick: Tick) -> Self {
        // SAFETY: These come from the same world. `world.components_registrator` can't be used since we borrow other fields too.
        let mut registrator =
            unsafe { ComponentsRegistrator::new(&mut world.components, &mut world.component_ids) };
        let bundle_id = world
            .bundles
            .register_info::<T>(&mut registrator, &mut world.storages);
        // SAFETY: we initialized this bundle_id in `init_info`
        unsafe { Self::new_with_id(world, bundle_id, change_tick) }
    }

    /// Creates a new [`BundleSpawner`].
    ///
    /// # Safety
    /// Caller must ensure that `bundle_id` exists in `world.bundles`
    #[inline]
    pub(crate) unsafe fn new_with_id(
        world: &'w mut World,
        bundle_id: BundleId,
        change_tick: Tick,
    ) -> Self {
        let bundle_info = world.bundles.get_unchecked(bundle_id);
        let (new_archetype_id, is_new_created) = bundle_info.insert_bundle_into_archetype(
            &mut world.archetypes,
            &mut world.storages,
            &world.components,
            &world.observers,
            ArchetypeId::EMPTY,
        );

        let archetype = &mut world.archetypes[new_archetype_id];
        let table = &mut world.storages.tables[archetype.table_id()];
        let spawner = Self {
            bundle_info: bundle_info.into(),
            table: table.into(),
            archetype: archetype.into(),
            change_tick,
            world: world.as_unsafe_world_cell(),
        };
        if is_new_created {
            spawner
                .world
                .into_deferred()
                .trigger(ArchetypeCreated(new_archetype_id));
        }
        spawner
    }

    #[inline]
    pub fn reserve_storage(&mut self, additional: usize) {
        // SAFETY: There are no outstanding world references
        let (archetype, table) = unsafe { (self.archetype.as_mut(), self.table.as_mut()) };
        archetype.reserve(additional);
        table.reserve(additional);
    }

    /// # Safety
    /// `entity` must be allocated (but non-existent), `T` must match this [`BundleInfo`]'s type
    #[inline]
    #[track_caller]
    pub unsafe fn spawn_non_existent<T: DynamicBundle>(
        &mut self,
        entity: Entity,
        bundle: T,
        caller: MaybeLocation,
    ) -> (EntityLocation, T::Effect) {
        // SAFETY: We do not make any structural changes to the archetype graph through self.world so these pointers always remain valid
        let bundle_info = self.bundle_info.as_ref();
        let (location, after_effect) = {
            let table = self.table.as_mut();
            let archetype = self.archetype.as_mut();

            // SAFETY: Mutable references do not alias and will be dropped after this block
            let (sparse_sets, entities) = {
                let world = self.world.world_mut();
                (&mut world.storages.sparse_sets, &mut world.entities)
            };
            let table_row = table.allocate(entity);
            let location = archetype.allocate(entity, table_row);
            let after_effect = bundle_info.write_components(
                table,
                sparse_sets,
                &SpawnBundleStatus,
                bundle_info.required_components.iter(),
                entity,
                table_row,
                self.change_tick,
                bundle,
                InsertMode::Replace,
                caller,
            );
            entities.set(entity.index(), Some(location));
            entities.mark_spawn_despawn(entity.index(), caller, self.change_tick);
            (location, after_effect)
        };

        // SAFETY: We have no outstanding mutable references to world as they were dropped
        let mut deferred_world = unsafe { self.world.into_deferred() };
        // SAFETY: `DeferredWorld` cannot provide mutable access to `Archetypes`.
        let archetype = self.archetype.as_ref();
        // SAFETY: All components in the bundle are guaranteed to exist in the World
        // as they must be initialized before creating the BundleInfo.
        unsafe {
            deferred_world.trigger_on_add(
                archetype,
                entity,
                bundle_info.iter_contributed_components(),
                caller,
            );
            if archetype.has_add_observer() {
                deferred_world.trigger_observers(
                    ADD,
                    Some(entity),
                    bundle_info.iter_contributed_components(),
                    caller,
                );
            }
            deferred_world.trigger_on_insert(
                archetype,
                entity,
                bundle_info.iter_contributed_components(),
                caller,
                RelationshipHookMode::Run,
            );
            if archetype.has_insert_observer() {
                deferred_world.trigger_observers(
                    INSERT,
                    Some(entity),
                    bundle_info.iter_contributed_components(),
                    caller,
                );
            }
        };

        (location, after_effect)
    }

    /// # Safety
    /// `T` must match this [`BundleInfo`]'s type
    #[inline]
    pub unsafe fn spawn<T: Bundle>(
        &mut self,
        bundle: T,
        caller: MaybeLocation,
    ) -> (Entity, T::Effect) {
        let entity = self.entities().alloc();
        // SAFETY: entity is allocated (but non-existent), `T` matches this BundleInfo's type
        let (_, after_effect) = unsafe { self.spawn_non_existent(entity, bundle, caller) };
        (entity, after_effect)
    }

    #[inline]
    pub(crate) fn entities(&mut self) -> &mut Entities {
        // SAFETY: No outstanding references to self.world, changes to entities cannot invalidate our internal pointers
        unsafe { &mut self.world.world_mut().entities }
    }

    /// # Safety
    /// - `Self` must be dropped after running this function as it may invalidate internal pointers.
    #[inline]
    pub(crate) unsafe fn flush_commands(&mut self) {
        // SAFETY: pointers on self can be invalidated,
        self.world.world_mut().flush();
    }
}

/// Metadata for bundles. Stores a [`BundleInfo`] for each type of [`Bundle`] in a given world.
#[derive(Default)]
pub struct Bundles {
    bundle_infos: Vec<BundleInfo>,
    /// Cache static [`BundleId`]
    bundle_ids: TypeIdMap<BundleId>,
    /// Cache bundles, which contains both explicit and required components of [`Bundle`]
    contributed_bundle_ids: TypeIdMap<BundleId>,
    /// Cache dynamic [`BundleId`] with multiple components
    dynamic_bundle_ids: HashMap<Box<[ComponentId]>, BundleId>,
    dynamic_bundle_storages: HashMap<BundleId, Vec<StorageType>>,
    /// Cache optimized dynamic [`BundleId`] with single component
    dynamic_component_bundle_ids: HashMap<ComponentId, BundleId>,
    dynamic_component_storages: HashMap<BundleId, StorageType>,
}

impl Bundles {
    /// The total number of [`Bundle`] registered in [`Storages`].
    pub fn len(&self) -> usize {
        self.bundle_infos.len()
    }

    /// Returns true if no [`Bundle`] registered in [`Storages`].
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Iterate over [`BundleInfo`].
    pub fn iter(&self) -> impl Iterator<Item = &BundleInfo> {
        self.bundle_infos.iter()
    }

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

    /// Registers a new [`BundleInfo`] for a statically known type.
    ///
    /// Also registers all the components in the bundle.
    pub(crate) fn register_info<T: Bundle>(
        &mut self,
        components: &mut ComponentsRegistrator,
        storages: &mut Storages,
    ) -> BundleId {
        let bundle_infos = &mut self.bundle_infos;
        *self.bundle_ids.entry(TypeId::of::<T>()).or_insert_with(|| {
            let mut component_ids= Vec::new();
            T::component_ids(components, &mut |id| component_ids.push(id));
            let id = BundleId(bundle_infos.len());
            let bundle_info =
                // SAFETY: T::component_id ensures:
                // - its info was created
                // - appropriate storage for it has been initialized.
                // - it was created in the same order as the components in T
                unsafe { BundleInfo::new(core::any::type_name::<T>(), storages, components, component_ids, id) };
            bundle_infos.push(bundle_info);
            id
        })
    }

    /// Registers a new [`BundleInfo`], which contains both explicit and required components for a statically known type.
    ///
    /// Also registers all the components in the bundle.
    pub(crate) fn register_contributed_bundle_info<T: Bundle>(
        &mut self,
        components: &mut ComponentsRegistrator,
        storages: &mut Storages,
    ) -> BundleId {
        if let Some(id) = self.contributed_bundle_ids.get(&TypeId::of::<T>()).cloned() {
            id
        } else {
            let explicit_bundle_id = self.register_info::<T>(components, storages);
            // SAFETY: reading from `explicit_bundle_id` and creating new bundle in same time. Its valid because bundle hashmap allow this
            let id = unsafe {
                let (ptr, len) = {
                    // SAFETY: `explicit_bundle_id` is valid and defined above
                    let contributed = self
                        .get_unchecked(explicit_bundle_id)
                        .contributed_components();
                    (contributed.as_ptr(), contributed.len())
                };
                // SAFETY: this is sound because the contributed_components Vec for explicit_bundle_id will not be accessed mutably as
                // part of init_dynamic_info. No mutable references will be created and the allocation will remain valid.
                self.init_dynamic_info(storages, components, core::slice::from_raw_parts(ptr, len))
            };
            self.contributed_bundle_ids.insert(TypeId::of::<T>(), id);
            id
        }
    }

    /// # Safety
    /// A [`BundleInfo`] with the given [`BundleId`] must have been initialized for this instance of `Bundles`.
    pub(crate) unsafe fn get_unchecked(&self, id: BundleId) -> &BundleInfo {
        self.bundle_infos.get_unchecked(id.0)
    }

    /// # Safety
    /// This [`BundleId`] must have been initialized with a single [`Component`] (via [`init_component_info`](Self::init_dynamic_info))
    pub(crate) unsafe fn get_storage_unchecked(&self, id: BundleId) -> StorageType {
        *self
            .dynamic_component_storages
            .get(&id)
            .debug_checked_unwrap()
    }

    /// # Safety
    /// This [`BundleId`] must have been initialized with multiple [`Component`]s (via [`init_dynamic_info`](Self::init_dynamic_info))
    pub(crate) unsafe fn get_storages_unchecked(&mut self, id: BundleId) -> &mut Vec<StorageType> {
        self.dynamic_bundle_storages
            .get_mut(&id)
            .debug_checked_unwrap()
    }

    /// Initializes a new [`BundleInfo`] for a dynamic [`Bundle`].
    ///
    /// # Panics
    ///
    /// Panics if any of the provided [`ComponentId`]s do not exist in the
    /// provided [`Components`].
    pub(crate) fn init_dynamic_info(
        &mut self,
        storages: &mut Storages,
        components: &Components,
        component_ids: &[ComponentId],
    ) -> BundleId {
        let bundle_infos = &mut self.bundle_infos;

        // Use `raw_entry_mut` to avoid cloning `component_ids` to access `Entry`
        let (_, bundle_id) = self
            .dynamic_bundle_ids
            .raw_entry_mut()
            .from_key(component_ids)
            .or_insert_with(|| {
                let (id, storages) = initialize_dynamic_bundle(
                    bundle_infos,
                    storages,
                    components,
                    Vec::from(component_ids),
                );
                // SAFETY: The ID always increases when new bundles are added, and so, the ID is unique.
                unsafe {
                    self.dynamic_bundle_storages
                        .insert_unique_unchecked(id, storages);
                }
                (component_ids.into(), id)
            });
        *bundle_id
    }

    /// Initializes a new [`BundleInfo`] for a dynamic [`Bundle`] with single component.
    ///
    /// # Panics
    ///
    /// Panics if the provided [`ComponentId`] does not exist in the provided [`Components`].
    pub(crate) fn init_component_info(
        &mut self,
        storages: &mut Storages,
        components: &Components,
        component_id: ComponentId,
    ) -> BundleId {
        let bundle_infos = &mut self.bundle_infos;
        let bundle_id = self
            .dynamic_component_bundle_ids
            .entry(component_id)
            .or_insert_with(|| {
                let (id, storage_type) = initialize_dynamic_bundle(
                    bundle_infos,
                    storages,
                    components,
                    vec![component_id],
                );
                self.dynamic_component_storages.insert(id, storage_type[0]);
                id
            });
        *bundle_id
    }
}

/// Asserts that all components are part of [`Components`]
/// and initializes a [`BundleInfo`].
fn initialize_dynamic_bundle(
    bundle_infos: &mut Vec<BundleInfo>,
    storages: &mut Storages,
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
        unsafe { BundleInfo::new("<dynamic bundle>", storages, components, component_ids, id) };
    bundle_infos.push(bundle_info);

    (id, storage_types)
}

fn sorted_remove<T: Eq + Ord + Copy>(source: &mut Vec<T>, remove: &[T]) {
    let mut remove_index = 0;
    source.retain(|value| {
        while remove_index < remove.len() && *value > remove[remove_index] {
            remove_index += 1;
        }

        if remove_index < remove.len() {
            *value != remove[remove_index]
        } else {
            true
        }
    });
}

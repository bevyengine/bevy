use alloc::{boxed::Box, vec, vec::Vec};
use bevy_platform::collections::{HashMap, HashSet};
use bevy_ptr::OwningPtr;
use bevy_utils::TypeIdMap;
use core::{any::TypeId, ptr::NonNull};

use crate::{
    archetype::{Archetype, BundleComponentStatus, ComponentStatus},
    bundle::{Bundle, DynamicBundle},
    change_detection::MaybeLocation,
    component::{
        ComponentId, Components, ComponentsRegistrator, RequiredComponentConstructor,
        RequiredComponents, StorageType, Tick,
    },
    entity::Entity,
    query::DebugCheckedUnwrap as _,
    storage::{SparseSetIndex, SparseSets, Storages, Table, TableRow},
};

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
    pub(super) id: BundleId,
    /// The list of all components contributed by the bundle (including Required Components). This is in
    /// the order `[EXPLICIT_COMPONENTS][REQUIRED_COMPONENTS]`
    ///
    /// # Safety
    /// Every ID in this list must be valid within the World that owns the [`BundleInfo`],
    /// must have its storage initialized (i.e. columns created in tables, sparse set created),
    /// and the range (0..`explicit_components_len`) must be in the same order as the source bundle
    /// type writes its components in.
    pub(super) component_ids: Vec<ComponentId>,
    pub(super) required_components: Vec<RequiredComponentConstructor>,
    pub(super) explicit_components_len: usize,
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
    /// When "inserting" a bundle into an existing entity, [`ArchetypeAfterBundleInsert`](crate::archetype::SpawnBundleStatus)
    /// should be used, which will report `Added` vs `Existing` status based on the current archetype's structure.
    ///
    /// When spawning a bundle, [`SpawnBundleStatus`](crate::archetype::SpawnBundleStatus) can be used instead,
    /// which removes the need to look up the [`ArchetypeAfterBundleInsert`](crate::archetype::ArchetypeAfterBundleInsert)
    /// in the archetype graph, which requires ownership of the entity's current archetype.
    ///
    /// `table` must be the "new" table for `entity`. `table_row` must have space allocated for the
    /// `entity`, `bundle` must match this [`BundleInfo`]'s type
    #[inline]
    pub(super) unsafe fn write_components<'a, T: DynamicBundle, S: BundleComponentStatus>(
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
    /// This [`BundleId`] must have been initialized with a single [`Component`](crate::component::Component)
    /// (via [`init_component_info`](Self::init_dynamic_info))
    pub(crate) unsafe fn get_storage_unchecked(&self, id: BundleId) -> StorageType {
        *self
            .dynamic_component_storages
            .get(&id)
            .debug_checked_unwrap()
    }

    /// # Safety
    /// This [`BundleId`] must have been initialized with multiple [`Component`](crate::component::Component)s
    /// (via [`init_dynamic_info`](Self::init_dynamic_info))
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

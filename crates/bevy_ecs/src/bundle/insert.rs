use alloc::vec::Vec;
use bevy_ptr::{ConstNonNull, MovingPtr};
use core::ptr::NonNull;

use crate::{
    archetype::{
        Archetype, ArchetypeAfterBundleInsert, ArchetypeCreated, ArchetypeId, Archetypes,
        ComponentStatus,
    },
    bundle::{ArchetypeMoveType, Bundle, BundleId, BundleInfo, DynamicBundle, InsertMode},
    change_detection::MaybeLocation,
    component::{Components, StorageType, Tick},
    entity::{Entities, Entity, EntityLocation},
    event::EntityComponentsTrigger,
    lifecycle::{Add, Insert, Replace, ADD, INSERT, REPLACE},
    observer::Observers,
    query::DebugCheckedUnwrap as _,
    relationship::RelationshipHookMode,
    storage::{Storages, Table},
    world::{unsafe_world_cell::UnsafeWorldCell, World},
};

// SAFETY: We have exclusive world access so our pointers can't be invalidated externally
pub(crate) struct BundleInserter<'w> {
    world: UnsafeWorldCell<'w>,
    bundle_info: ConstNonNull<BundleInfo>,
    archetype_after_insert: ConstNonNull<ArchetypeAfterBundleInsert>,
    table: NonNull<Table>,
    archetype: NonNull<Archetype>,
    archetype_move_type: ArchetypeMoveType,
    change_tick: Tick,
}

impl<'w> BundleInserter<'w> {
    #[inline]
    pub(crate) fn new<T: Bundle>(
        world: &'w mut World,
        archetype_id: ArchetypeId,
        change_tick: Tick,
    ) -> Self {
        let bundle_id = world.register_bundle_info::<T>();

        // SAFETY: We just ensured this bundle exists
        unsafe { Self::new_with_id(world, archetype_id, bundle_id, change_tick) }
    }

    /// Creates a new [`BundleInserter`].
    ///
    /// # Safety
    /// - Caller must ensure that `bundle_id` exists in `world.bundles`.
    #[inline]
    pub(crate) unsafe fn new_with_id(
        world: &'w mut World,
        archetype_id: ArchetypeId,
        bundle_id: BundleId,
        change_tick: Tick,
    ) -> Self {
        // SAFETY: We will not make any accesses to the command queue, component or resource data of this world
        let bundle_info = world.bundles.get_unchecked(bundle_id);
        let bundle_id = bundle_info.id();
        let (new_archetype_id, is_new_created) = bundle_info.insert_bundle_into_archetype(
            &mut world.archetypes,
            &mut world.storages,
            &world.components,
            &world.observers,
            archetype_id,
        );

        let inserter = if new_archetype_id == archetype_id {
            let archetype = &mut world.archetypes[archetype_id];
            // SAFETY: The edge is assured to be initialized when we called insert_bundle_into_archetype
            let archetype_after_insert = unsafe {
                archetype
                    .edges()
                    .get_archetype_after_bundle_insert_internal(bundle_id)
                    .debug_checked_unwrap()
            };
            let table_id = archetype.table_id();
            let table = &mut world.storages.tables[table_id];
            Self {
                archetype_after_insert: archetype_after_insert.into(),
                archetype: archetype.into(),
                bundle_info: bundle_info.into(),
                table: table.into(),
                archetype_move_type: ArchetypeMoveType::SameArchetype,
                change_tick,
                world: world.as_unsafe_world_cell(),
            }
        } else {
            let (archetype, new_archetype) =
                world.archetypes.get_2_mut(archetype_id, new_archetype_id);
            // SAFETY: The edge is assured to be initialized when we called insert_bundle_into_archetype
            let archetype_after_insert = unsafe {
                archetype
                    .edges()
                    .get_archetype_after_bundle_insert_internal(bundle_id)
                    .debug_checked_unwrap()
            };
            let table_id = archetype.table_id();
            let new_table_id = new_archetype.table_id();
            if table_id == new_table_id {
                let table = &mut world.storages.tables[table_id];
                Self {
                    archetype_after_insert: archetype_after_insert.into(),
                    archetype: archetype.into(),
                    bundle_info: bundle_info.into(),
                    table: table.into(),
                    archetype_move_type: ArchetypeMoveType::NewArchetypeSameTable {
                        new_archetype: new_archetype.into(),
                    },
                    change_tick,
                    world: world.as_unsafe_world_cell(),
                }
            } else {
                let (table, new_table) = world.storages.tables.get_2_mut(table_id, new_table_id);
                Self {
                    archetype_after_insert: archetype_after_insert.into(),
                    archetype: archetype.into(),
                    bundle_info: bundle_info.into(),
                    table: table.into(),
                    archetype_move_type: ArchetypeMoveType::NewArchetypeNewTable {
                        new_archetype: new_archetype.into(),
                        new_table: new_table.into(),
                    },
                    change_tick,
                    world: world.as_unsafe_world_cell(),
                }
            }
        };

        if is_new_created {
            inserter
                .world
                .into_deferred()
                .trigger(ArchetypeCreated(new_archetype_id));
        }
        inserter
    }

    /// # Safety
    /// - `entity` must currently exist in the source archetype for this inserter.
    /// - `location` must be `entity`'s location in the archetype.
    /// - `T` must match this [`BundleInserter`] type used to create
    /// - If `T::Effect: !NoBundleEffect.`, then [`apply_effect`] must be called at most once on
    ///   `bundle` after this function before returning to user-space safe code.
    /// - The value pointed to by `bundle` must not be accessed for anything other than [`apply_effect`]
    ///   or dropped.
    ///
    /// [`apply_effect`]: crate::bundle::DynamicBundle::apply_effect
    #[inline]
    pub(crate) unsafe fn insert<T: DynamicBundle>(
        &mut self,
        entity: Entity,
        location: EntityLocation,
        bundle: MovingPtr<'_, T>,
        insert_mode: InsertMode,
        caller: MaybeLocation,
        relationship_hook_mode: RelationshipHookMode,
    ) -> EntityLocation {
        let bundle_info = self.bundle_info.as_ref();
        let archetype_after_insert = self.archetype_after_insert.as_ref();
        let archetype = self.archetype.as_ref();

        // SAFETY: All components in the bundle are guaranteed to exist in the World
        // as they must be initialized before creating the BundleInfo.
        unsafe {
            // SAFETY: Mutable references do not alias and will be dropped after this block
            let mut deferred_world = self.world.into_deferred();

            if insert_mode == InsertMode::Replace {
                if archetype.has_replace_observer() {
                    // SAFETY: the REPLACE event_key corresponds to the Replace event's type
                    deferred_world.trigger_raw(
                        REPLACE,
                        &mut Replace { entity },
                        &mut EntityComponentsTrigger {
                            components: &archetype_after_insert.existing,
                        },
                        caller,
                    );
                }
                deferred_world.trigger_on_replace(
                    archetype,
                    entity,
                    archetype_after_insert.iter_existing(),
                    caller,
                    relationship_hook_mode,
                );
            }
        }

        let table = self.table.as_mut();

        // SAFETY: Archetype gets borrowed when running the on_replace observers above,
        // so this reference can only be promoted from shared to &mut down here, after they have been ran
        let archetype = self.archetype.as_mut();

        let (new_archetype, new_location) = match &mut self.archetype_move_type {
            ArchetypeMoveType::SameArchetype => {
                // SAFETY: Mutable references do not alias and will be dropped after this block
                let sparse_sets = {
                    let world = self.world.world_mut();
                    &mut world.storages.sparse_sets
                };

                bundle_info.write_components(
                    table,
                    sparse_sets,
                    archetype_after_insert,
                    archetype_after_insert.required_components.iter(),
                    entity,
                    location.table_row,
                    self.change_tick,
                    bundle,
                    insert_mode,
                    caller,
                );

                (archetype, location)
            }
            ArchetypeMoveType::NewArchetypeSameTable { new_archetype } => {
                let new_archetype = new_archetype.as_mut();

                // SAFETY: Mutable references do not alias and will be dropped after this block
                let (sparse_sets, entities) = {
                    let world = self.world.world_mut();
                    (&mut world.storages.sparse_sets, &mut world.entities)
                };

                let result = archetype.swap_remove(location.archetype_row);
                if let Some(swapped_entity) = result.swapped_entity {
                    let swapped_location =
                        // SAFETY: If the swap was successful, swapped_entity must be valid.
                        unsafe { entities.get(swapped_entity).debug_checked_unwrap() };
                    entities.set(
                        swapped_entity.index(),
                        Some(EntityLocation {
                            archetype_id: swapped_location.archetype_id,
                            archetype_row: location.archetype_row,
                            table_id: swapped_location.table_id,
                            table_row: swapped_location.table_row,
                        }),
                    );
                }
                let new_location = new_archetype.allocate(entity, result.table_row);
                entities.set(entity.index(), Some(new_location));
                bundle_info.write_components(
                    table,
                    sparse_sets,
                    archetype_after_insert,
                    archetype_after_insert.required_components.iter(),
                    entity,
                    result.table_row,
                    self.change_tick,
                    bundle,
                    insert_mode,
                    caller,
                );

                (new_archetype, new_location)
            }
            ArchetypeMoveType::NewArchetypeNewTable {
                new_archetype,
                new_table,
            } => {
                let new_table = new_table.as_mut();
                let new_archetype = new_archetype.as_mut();

                // SAFETY: Mutable references do not alias and will be dropped after this block
                let (archetypes_ptr, sparse_sets, entities) = {
                    let world = self.world.world_mut();
                    let archetype_ptr: *mut Archetype = world.archetypes.archetypes.as_mut_ptr();
                    (
                        archetype_ptr,
                        &mut world.storages.sparse_sets,
                        &mut world.entities,
                    )
                };
                let result = archetype.swap_remove(location.archetype_row);
                if let Some(swapped_entity) = result.swapped_entity {
                    let swapped_location =
                        // SAFETY: If the swap was successful, swapped_entity must be valid.
                        unsafe { entities.get(swapped_entity).debug_checked_unwrap() };
                    entities.set(
                        swapped_entity.index(),
                        Some(EntityLocation {
                            archetype_id: swapped_location.archetype_id,
                            archetype_row: location.archetype_row,
                            table_id: swapped_location.table_id,
                            table_row: swapped_location.table_row,
                        }),
                    );
                }
                // PERF: store "non bundle" components in edge, then just move those to avoid
                // redundant copies
                let move_result = table.move_to_superset_unchecked(result.table_row, new_table);
                let new_location = new_archetype.allocate(entity, move_result.new_row);
                entities.set(entity.index(), Some(new_location));

                // If an entity was moved into this entity's table spot, update its table row.
                if let Some(swapped_entity) = move_result.swapped_entity {
                    let swapped_location =
                        // SAFETY: If the swap was successful, swapped_entity must be valid.
                        unsafe { entities.get(swapped_entity).debug_checked_unwrap() };

                    entities.set(
                        swapped_entity.index(),
                        Some(EntityLocation {
                            archetype_id: swapped_location.archetype_id,
                            archetype_row: swapped_location.archetype_row,
                            table_id: swapped_location.table_id,
                            table_row: result.table_row,
                        }),
                    );

                    if archetype.id() == swapped_location.archetype_id {
                        archetype
                            .set_entity_table_row(swapped_location.archetype_row, result.table_row);
                    } else if new_archetype.id() == swapped_location.archetype_id {
                        new_archetype
                            .set_entity_table_row(swapped_location.archetype_row, result.table_row);
                    } else {
                        // SAFETY: the only two borrowed archetypes are above and we just did collision checks
                        (*archetypes_ptr.add(swapped_location.archetype_id.index()))
                            .set_entity_table_row(swapped_location.archetype_row, result.table_row);
                    }
                }

                bundle_info.write_components(
                    new_table,
                    sparse_sets,
                    archetype_after_insert,
                    archetype_after_insert.required_components.iter(),
                    entity,
                    move_result.new_row,
                    self.change_tick,
                    bundle,
                    insert_mode,
                    caller,
                );

                (new_archetype, new_location)
            }
        };

        let new_archetype = &*new_archetype;
        // SAFETY: We have no outstanding mutable references to world as they were dropped
        let mut deferred_world = unsafe { self.world.into_deferred() };

        // SAFETY: All components in the bundle are guaranteed to exist in the World
        // as they must be initialized before creating the BundleInfo.
        unsafe {
            deferred_world.trigger_on_add(
                new_archetype,
                entity,
                archetype_after_insert.iter_added(),
                caller,
            );
            if new_archetype.has_add_observer() {
                // SAFETY: the ADD event_key corresponds to the Add event's type
                deferred_world.trigger_raw(
                    ADD,
                    &mut Add { entity },
                    &mut EntityComponentsTrigger {
                        components: &archetype_after_insert.added,
                    },
                    caller,
                );
            }
            match insert_mode {
                InsertMode::Replace => {
                    // Insert triggers for both new and existing components if we're replacing them.
                    deferred_world.trigger_on_insert(
                        new_archetype,
                        entity,
                        archetype_after_insert.iter_inserted(),
                        caller,
                        relationship_hook_mode,
                    );
                    if new_archetype.has_insert_observer() {
                        // SAFETY: the INSERT event_key corresponds to the Insert event's type
                        deferred_world.trigger_raw(
                            INSERT,
                            &mut Insert { entity },
                            // PERF: this is not a regression from what we were doing before, but ideally we don't
                            // need to collect here
                            &mut EntityComponentsTrigger {
                                components: &archetype_after_insert
                                    .iter_inserted()
                                    .collect::<Vec<_>>(),
                            },
                            caller,
                        );
                    }
                }
                InsertMode::Keep => {
                    // Insert triggers only for new components if we're not replacing them (since
                    // nothing is actually inserted).
                    deferred_world.trigger_on_insert(
                        new_archetype,
                        entity,
                        archetype_after_insert.iter_added(),
                        caller,
                        relationship_hook_mode,
                    );
                    if new_archetype.has_insert_observer() {
                        // SAFETY: the INSERT event_key corresponds to the Insert event's type
                        deferred_world.trigger_raw(
                            INSERT,
                            &mut Insert { entity },
                            &mut EntityComponentsTrigger {
                                components: &archetype_after_insert.added,
                            },
                            caller,
                        );
                    }
                }
            }
        }

        new_location
    }

    #[inline]
    pub(crate) fn entities(&mut self) -> &mut Entities {
        // SAFETY: No outstanding references to self.world, changes to entities cannot invalidate our internal pointers
        unsafe { &mut self.world.world_mut().entities }
    }
}

impl BundleInfo {
    /// Inserts a bundle into the given archetype and returns the resulting archetype and whether a new archetype was created.
    /// This could be the same [`ArchetypeId`], in the event that inserting the given bundle
    /// does not result in an [`Archetype`] change.
    ///
    /// Results are cached in the [`Archetype`] graph to avoid redundant work.
    ///
    /// # Safety
    /// `components` must be the same components as passed in [`Self::new`]
    pub(crate) unsafe fn insert_bundle_into_archetype(
        &self,
        archetypes: &mut Archetypes,
        storages: &mut Storages,
        components: &Components,
        observers: &Observers,
        archetype_id: ArchetypeId,
    ) -> (ArchetypeId, bool) {
        if let Some(archetype_after_insert_id) = archetypes[archetype_id]
            .edges()
            .get_archetype_after_bundle_insert(self.id)
        {
            return (archetype_after_insert_id, false);
        }
        let mut new_table_components = Vec::new();
        let mut new_sparse_set_components = Vec::new();
        let mut bundle_status = Vec::with_capacity(self.explicit_components_len());
        let mut added_required_components = Vec::new();
        let mut added = Vec::new();
        let mut existing = Vec::new();

        let current_archetype = &mut archetypes[archetype_id];
        for component_id in self.iter_explicit_components() {
            if current_archetype.contains(component_id) {
                bundle_status.push(ComponentStatus::Existing);
                existing.push(component_id);
            } else {
                bundle_status.push(ComponentStatus::Added);
                added.push(component_id);
                // SAFETY: component_id exists
                let component_info = unsafe { components.get_info_unchecked(component_id) };
                match component_info.storage_type() {
                    StorageType::Table => new_table_components.push(component_id),
                    StorageType::SparseSet => new_sparse_set_components.push(component_id),
                }
            }
        }

        for (index, component_id) in self.iter_required_components().enumerate() {
            if !current_archetype.contains(component_id) {
                added_required_components.push(self.required_component_constructors[index].clone());
                added.push(component_id);
                // SAFETY: component_id exists
                let component_info = unsafe { components.get_info_unchecked(component_id) };
                match component_info.storage_type() {
                    StorageType::Table => {
                        new_table_components.push(component_id);
                    }
                    StorageType::SparseSet => {
                        new_sparse_set_components.push(component_id);
                    }
                }
            }
        }

        if new_table_components.is_empty() && new_sparse_set_components.is_empty() {
            let edges = current_archetype.edges_mut();
            // The archetype does not change when we insert this bundle.
            edges.cache_archetype_after_bundle_insert(
                self.id,
                archetype_id,
                bundle_status,
                added_required_components,
                added,
                existing,
            );
            (archetype_id, false)
        } else {
            let table_id;
            let table_components;
            let sparse_set_components;
            // The archetype changes when we insert this bundle. Prepare the new archetype and storages.
            {
                let current_archetype = &archetypes[archetype_id];
                table_components = if new_table_components.is_empty() {
                    // If there are no new table components, we can keep using this table.
                    table_id = current_archetype.table_id();
                    current_archetype.table_components().collect()
                } else {
                    new_table_components.extend(current_archetype.table_components());
                    // Sort to ignore order while hashing.
                    new_table_components.sort_unstable();
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
                    // Sort to ignore order while hashing.
                    new_sparse_set_components.sort_unstable();
                    new_sparse_set_components
                };
            };
            // SAFETY: ids in self must be valid
            let (new_archetype_id, is_new_created) = archetypes.get_id_or_insert(
                components,
                observers,
                table_id,
                table_components,
                sparse_set_components,
            );

            // Add an edge from the old archetype to the new archetype.
            archetypes[archetype_id]
                .edges_mut()
                .cache_archetype_after_bundle_insert(
                    self.id,
                    new_archetype_id,
                    bundle_status,
                    added_required_components,
                    added,
                    existing,
                );
            (new_archetype_id, is_new_created)
        }
    }
}

use alloc::vec::Vec;
use bevy_ptr::ConstNonNull;
use core::ptr::NonNull;

use crate::{
    archetype::{Archetype, ArchetypeCreated, ArchetypeId, Archetypes},
    bundle::{Bundle, BundleId, BundleInfo},
    change_detection::MaybeLocation,
    component::{ComponentId, Components, StorageType},
    entity::{Entity, EntityLocation},
    event::EntityComponentsTrigger,
    lifecycle::{AfterRemove, Discard, Remove, AFTER_REMOVE, DISCARD, REMOVE},
    observer::Observers,
    relationship::RelationshipHookMode,
    storage::{SparseSets, Storages, Table},
    world::{unsafe_world_cell::UnsafeWorldCell, World},
};

// SAFETY: We have exclusive world access so our pointers can't be invalidated externally
pub(crate) struct BundleRemover<'w> {
    world: UnsafeWorldCell<'w>,
    bundle_info: ConstNonNull<BundleInfo>,
    old_and_new_table: Option<(NonNull<Table>, NonNull<Table>)>,
    old_archetype: NonNull<Archetype>,
    new_archetype: NonNull<Archetype>,
    pub(crate) relationship_hook_mode: RelationshipHookMode,
}

#[inline]
fn explicit_components_in_archetype<'a>(
    explicit_components: &'a [ComponentId],
    archetype: &'a Archetype,
) -> impl Iterator<Item = ComponentId> + 'a {
    explicit_components
        .iter()
        .copied()
        .filter(move |component_id| archetype.contains(*component_id))
}

impl<'w> BundleRemover<'w> {
    /// Creates a new [`BundleRemover`], if such a remover would do anything.
    ///
    /// If `require_all` is true, the [`BundleRemover`] is only created if the entire bundle is present on the archetype.
    ///
    /// # Safety
    /// Caller must ensure that `archetype_id` is valid
    #[inline]
    #[deny(unsafe_op_in_unsafe_fn)]
    pub(crate) unsafe fn new<T: Bundle>(
        world: &'w mut World,
        archetype_id: ArchetypeId,
        require_all: bool,
    ) -> Option<Self> {
        let bundle_id = world.register_bundle_info::<T>();

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
            relationship_hook_mode: RelationshipHookMode::Run,
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
        let bundle_info = self.bundle_info.as_ref();
        let explicit_components = bundle_info.explicit_components();
        let old_archetype_id = location.archetype_id;
        // Save flags before mutation invalidates archetype references.
        let has_after_remove_hook = self.old_archetype.as_ref().has_after_remove_hook();
        let has_after_remove_observer = self.old_archetype.as_ref().has_after_remove_observer();

        // SAFETY: all bundle components exist in World
        unsafe {
            let old_archetype = self.old_archetype.as_ref();
            let new_archetype = self.new_archetype.as_ref();
            let has_discard_observer = old_archetype.has_discard_observer();
            let has_remove_observer = old_archetype.has_remove_observer();
            let observer_components = (has_discard_observer || has_remove_observer).then(|| {
                explicit_components_in_archetype(explicit_components, old_archetype)
                    .collect::<Vec<_>>()
            });

            // SAFETY: We only keep access to archetype metadata.
            let mut deferred_world = self.world.into_deferred();
            if has_discard_observer {
                let components = observer_components.as_ref().unwrap();
                // SAFETY: the DISCARD event_key corresponds to the Discard event's type
                deferred_world.trigger_raw(
                    DISCARD,
                    &mut Discard { entity },
                    &mut EntityComponentsTrigger {
                        components,
                        old_archetype: Some(old_archetype),
                        new_archetype: Some(new_archetype),
                    },
                    caller,
                );
            }
            deferred_world.trigger_on_discard(
                old_archetype,
                entity,
                explicit_components_in_archetype(explicit_components, old_archetype),
                caller,
                self.relationship_hook_mode,
            );
            if has_remove_observer {
                let components = observer_components.as_ref().unwrap();
                // SAFETY: the REMOVE event_key corresponds to the Remove event's type
                deferred_world.trigger_raw(
                    REMOVE,
                    &mut Remove { entity },
                    &mut EntityComponentsTrigger {
                        components,
                        old_archetype: Some(old_archetype),
                        new_archetype: Some(new_archetype),
                    },
                    caller,
                );
            }
            deferred_world.trigger_on_remove(
                old_archetype,
                entity,
                explicit_components_in_archetype(explicit_components, old_archetype),
                caller,
            );
        }

        let (new_location, pre_remove_result) = {
            // SAFETY: We still have the cell, so this is unique, it doesn't conflict with other references, and we drop it shortly.
            let world = unsafe { self.world.world_mut() };

            let (needs_drop, pre_remove_result) = pre_remove(
                &mut world.storages.sparse_sets,
                self.old_and_new_table
                    .as_ref()
                    // SAFETY: There is no conflicting access for this scope.
                    .map(|(old, _)| unsafe { &mut *old.as_ptr() }),
                &world.components,
                explicit_components,
            );

            {
                let old_archetype = self.old_archetype.as_ref();
                for &component_id in explicit_components {
                    if !old_archetype.contains(component_id) {
                        continue;
                    }
                    world.removed_components.write(component_id, entity);

                    // Make sure to drop components stored in sparse sets.
                    // Dense components are dropped later in `move_to_and_drop_missing_unchecked`.
                    if let Some(StorageType::SparseSet) =
                        old_archetype.get_storage_type(component_id)
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

            let remove_result = self
                .old_archetype
                .as_mut()
                .swap_remove(location.archetype_row);
            if let Some(swapped_entity) = remove_result.swapped_entity {
                let swapped_location = world.entities.get_spawned(swapped_entity).unwrap();

                world.entities.update_existing_location(
                    swapped_entity.index(),
                    Some(EntityLocation {
                        archetype_id: swapped_location.archetype_id,
                        archetype_row: location.archetype_row,
                        table_id: swapped_location.table_id,
                        table_row: swapped_location.table_row,
                    }),
                );
            }

            let new_location = if let Some((mut old_table, mut new_table)) = self.old_and_new_table
            {
                let move_result = if needs_drop {
                    // SAFETY: old_table_row exists
                    unsafe {
                        old_table.as_mut().move_to_and_drop_missing_unchecked(
                            location.table_row,
                            new_table.as_mut(),
                        )
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
                let allocated_location = unsafe {
                    self.new_archetype
                        .as_mut()
                        .allocate(entity, move_result.new_row)
                };

                if let Some(swapped_entity) = move_result.swapped_entity {
                    let swapped_location = world.entities.get_spawned(swapped_entity).unwrap();

                    world.entities.update_existing_location(
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

                allocated_location
            } else {
                self.new_archetype
                    .as_mut()
                    .allocate(entity, location.table_row)
            };

            // SAFETY: The entity is valid and has been moved to the new location already.
            unsafe {
                world
                    .entities
                    .update_existing_location(entity.index(), Some(new_location));
            }

            (new_location, pre_remove_result)
        };

        // Trigger AfterRemove after the entity has moved and removed data is gone.
        // SAFETY:
        // - all bundle components exist in World
        // - only type metadata is read from archetypes during these hooks
        if has_after_remove_hook || has_after_remove_observer {
            // SAFETY:
            // - `old_archetype_id` and `new_location.archetype_id` are valid IDs
            // - only type metadata is read from archetypes while triggering hooks/observers
            unsafe {
                let world = self.world;
                let archetypes = world.archetypes();
                let old_archetype = &archetypes[old_archetype_id];
                let mut deferred_world = world.into_deferred();
                deferred_world.trigger_after_remove(
                    old_archetype,
                    entity,
                    explicit_components_in_archetype(explicit_components, old_archetype),
                    caller,
                );
                if has_after_remove_observer {
                    let new_archetype = &archetypes[new_location.archetype_id];
                    let removed_components =
                        explicit_components_in_archetype(explicit_components, old_archetype)
                            .collect::<Vec<_>>();
                    deferred_world.trigger_raw(
                        AFTER_REMOVE,
                        &mut AfterRemove { entity },
                        &mut EntityComponentsTrigger {
                            components: &removed_components,
                            old_archetype: Some(old_archetype),
                            new_archetype: Some(new_archetype),
                        },
                        caller,
                    );
                }
            }
        }

        (new_location, pre_remove_result)
    }
}

impl BundleInfo {
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

#[cfg(test)]
mod tests {
    use alloc::vec;

    #[test]
    fn sorted_remove() {
        let mut a = vec![1, 2, 3, 4, 5, 6, 7];
        let b = vec![1, 2, 3, 5, 7];
        super::sorted_remove(&mut a, &b);

        assert_eq!(a, vec![4, 6]);

        let mut a = vec![1];
        let b = vec![1];
        super::sorted_remove(&mut a, &b);

        assert_eq!(a, vec![]);

        let mut a = vec![1];
        let b = vec![2];
        super::sorted_remove(&mut a, &b);

        assert_eq!(a, vec![1]);
    }
}

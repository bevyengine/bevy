use core::debug_assert_matches;

use alloc::vec::Vec;
use nonmax::NonMaxU32;

use crate::{
    archetype::ArchetypeRow,
    change_detection::MaybeLocation,
    component::Component,
    entity::{Entity, EntityLocation},
    query::With,
    world::World,
};

impl World {
    /// Bulk despawn all entities with a given [`Component`]
    pub fn despawn_all_with<C: Component>(&mut self) {
        let caller = MaybeLocation::caller();

        let change_tick = self.change_tick();

        let Some(cid) = self.component_id::<C>() else {
            return;
        };

        let Some(arches) = self
            .archetypes
            .by_component
            .get(&cid)
            .map(|m| m.keys().copied().collect::<Vec<_>>())
        else {
            return;
        };

        // mostly adoped from EntityWorldMut::despawn_no_free_with_caller

        for arch_id in arches {
            let archetype = &mut self.archetypes[arch_id];

            let table_id = archetype.table_id();

            let entities = archetype.entities().to_vec();

            let components = archetype.components().to_vec();
            let sparse_components = archetype.sparse_set_components().collect::<Vec<_>>();

            for (idx, entity) in entities.iter().rev().enumerate() {
                let archetype_row =
                    unsafe { ArchetypeRow::new(NonMaxU32::new_unchecked(idx as u32)) };

                {
                    // TODO:
                    //   despawn_observer
                    //   discard_observer
                    //   remove_observer
                }

                for component_id in &components {
                    self.removed_components.write(*component_id, entity.id());
                }

                unsafe {
                    self.entities
                        .update_existing_location(entity.id().index(), None);
                    self.entities.mark_spawned_or_despawned(
                        entity.id().index(),
                        caller,
                        change_tick,
                    );
                }

                let remove_result = self.archetypes[arch_id].swap_remove(archetype_row);

                // Assuming there is no swapped entity because we're iterating the entities in reverse order
                debug_assert_matches!(remove_result.swapped_entity, None);

                // SAFETY ?? skipping world.entities.update_existing_location

                for component_id in &sparse_components {
                    // set must have existed for the component to be added.
                    let sparse_set = self.storages.sparse_sets.get_mut(*component_id).unwrap();
                    sparse_set.remove(entity.id());
                }

                let moved_entity = unsafe {
                    self.storages.tables[table_id].swap_remove_unchecked(entity.table_row())
                };

                if let Some(moved_entity) = moved_entity {
                    let moved_location = self.entities.get_spawned(moved_entity).unwrap();
                    // SAFETY ??
                    unsafe {
                        self.entities.update_existing_location(
                            moved_entity.index(),
                            Some(EntityLocation {
                                archetype_id: moved_location.archetype_id,
                                archetype_row: moved_location.archetype_row,
                                table_id: moved_location.table_id,
                                table_row: entity.table_row(),
                            }),
                        );
                    }
                    self.archetypes[moved_location.archetype_id]
                        .set_entity_table_row(moved_location.archetype_row, entity.table_row());
                }

                unsafe {
                    self.entities.mark_free(entity.id().index(), 1);
                }
            }
        }

        self.flush();
    }

    /// Bulk despawn all entities with a given [`Component`], naive implementation for comparison
    pub fn despawn_all_with_naive<C: Component>(&mut self) {
        let entities = self
            .query_filtered::<Entity, With<C>>()
            .iter(self)
            .collect::<Vec<_>>();

        for e in entities {
            self.despawn(e);
        }
    }
}

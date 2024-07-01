use crate::{
    component::{Component, ComponentHooks, StorageType},
    entity::Entity,
    observer::ObserverState,
};

/// Tracks a list of entity observers for the [`Entity`] [`ObservedBy`] is added to.
#[derive(Default)]
pub(crate) struct ObservedBy(pub(crate) Vec<Entity>);

impl Component for ObservedBy {
    const STORAGE_TYPE: StorageType = StorageType::SparseSet;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_remove(|mut world, entity, _| {
            let observed_by = {
                let mut component = world.get_mut::<ObservedBy>(entity).unwrap();
                std::mem::take(&mut component.0)
            };
            for e in observed_by {
                let (total_entities, despawned_watched_entities) = {
                    let Some(mut entity_mut) = world.get_entity_mut(e) else {
                        continue;
                    };
                    let Some(mut state) = entity_mut.get_mut::<ObserverState>() else {
                        continue;
                    };
                    state.despawned_watched_entities += 1;
                    (
                        state.descriptor.entities.len(),
                        state.despawned_watched_entities as usize,
                    )
                };

                // Despawn Observer if it has no more active sources.
                if total_entities == despawned_watched_entities {
                    world.commands().entity(e).despawn();
                }
            }
        });
    }
}

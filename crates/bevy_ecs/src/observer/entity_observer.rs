use crate::{
    component::{
        Component, ComponentCloneHandler, ComponentHooks, HookContext, Mutable, StorageType,
    },
    entity::{ComponentCloneCtx, Entity, EntityCloneBuilder},
    observer::ObserverState,
    world::{DeferredWorld, World},
};
use alloc::vec::Vec;

/// Tracks a list of entity observers for the [`Entity`] [`ObservedBy`] is added to.
#[derive(Default)]
pub struct ObservedBy(pub(crate) Vec<Entity>);

impl Component for ObservedBy {
    const STORAGE_TYPE: StorageType = StorageType::SparseSet;
    type Mutability = Mutable;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_remove(|mut world, HookContext { entity, .. }| {
            let observed_by = {
                let mut component = world.get_mut::<ObservedBy>(entity).unwrap();
                core::mem::take(&mut component.0)
            };
            for e in observed_by {
                let (total_entities, despawned_watched_entities) = {
                    let Ok(mut entity_mut) = world.get_entity_mut(e) else {
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

    fn get_component_clone_handler() -> ComponentCloneHandler {
        ComponentCloneHandler::ignore()
    }
}

/// Trait that holds functions for configuring interaction with observers during entity cloning.
pub trait CloneEntityWithObserversExt {
    /// Sets the option to automatically add cloned entities to the observers targeting source entity.
    fn add_observers(&mut self, add_observers: bool) -> &mut Self;
}

impl CloneEntityWithObserversExt for EntityCloneBuilder<'_> {
    fn add_observers(&mut self, add_observers: bool) -> &mut Self {
        if add_observers {
            self.override_component_clone_handler::<ObservedBy>(
                ComponentCloneHandler::custom_handler(component_clone_observed_by),
            )
        } else {
            self.remove_component_clone_handler_override::<ObservedBy>()
        }
    }
}

fn component_clone_observed_by(world: &mut DeferredWorld, ctx: &mut ComponentCloneCtx) {
    let target = ctx.target();
    let source = ctx.source();

    world.commands().queue(move |world: &mut World| {
        let observed_by = world
            .get::<ObservedBy>(source)
            .map(|observed_by| observed_by.0.clone())
            .expect("Source entity must have ObservedBy");

        world
            .entity_mut(target)
            .insert(ObservedBy(observed_by.clone()));

        for observer in &observed_by {
            let mut observer_state = world
                .get_mut::<ObserverState>(*observer)
                .expect("Source observer entity must have ObserverState");
            observer_state.descriptor.entities.push(target);
            let event_types = observer_state.descriptor.events.clone();
            let components = observer_state.descriptor.components.clone();
            for event_type in event_types {
                let observers = world.observers.get_observers(event_type);
                if components.is_empty() {
                    if let Some(map) = observers.entity_observers.get(&source).cloned() {
                        observers.entity_observers.insert(target, map);
                    }
                } else {
                    for component in &components {
                        let Some(observers) = observers.component_observers.get_mut(component)
                        else {
                            continue;
                        };
                        if let Some(map) = observers.entity_map.get(&source).cloned() {
                            observers.entity_map.insert(target, map);
                        }
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use crate::{
        self as bevy_ecs,
        entity::EntityCloneBuilder,
        event::Event,
        observer::{CloneEntityWithObserversExt, Trigger},
        resource::Resource,
        system::ResMut,
        world::World,
    };

    #[derive(Resource, Default)]
    struct Num(usize);

    #[derive(Event)]
    struct E;

    #[test]
    fn clone_entity_with_observer() {
        let mut world = World::default();
        world.init_resource::<Num>();

        let e = world
            .spawn_empty()
            .observe(|_: Trigger<E>, mut res: ResMut<Num>| res.0 += 1)
            .id();
        world.flush();

        world.trigger_targets(E, e);

        let e_clone = world.spawn_empty().id();
        let mut builder = EntityCloneBuilder::new(&mut world);
        builder.add_observers(true);
        builder.clone_entity(e, e_clone);

        world.trigger_targets(E, [e, e_clone]);

        assert_eq!(world.resource::<Num>().0, 3);
    }
}

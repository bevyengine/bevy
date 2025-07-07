//! Logic to track observers when cloning entities.

use crate::{
    component::ComponentCloneBehavior,
    entity::{
        CloneByFilter, ComponentCloneCtx, EntityClonerBuilder, EntityMapper, SourceComponent,
    },
    observer::ObservedBy,
    world::World,
};

use super::Observer;

impl<Filter: CloneByFilter> EntityClonerBuilder<'_, Filter> {
    /// Sets the option to automatically add cloned entities to the observers targeting source entity.
    pub fn add_observers(&mut self, add_observers: bool) -> &mut Self {
        if add_observers {
            self.override_clone_behavior::<ObservedBy>(ComponentCloneBehavior::Custom(
                component_clone_observed_by,
            ))
        } else {
            self.remove_clone_behavior_override::<ObservedBy>()
        }
    }
}

fn component_clone_observed_by(_source: &SourceComponent, ctx: &mut ComponentCloneCtx) {
    let target = ctx.target();
    let source = ctx.source();

    ctx.queue_deferred(move |world: &mut World, _mapper: &mut dyn EntityMapper| {
        let observed_by = world
            .get::<ObservedBy>(source)
            .map(|observed_by| observed_by.0.clone())
            .expect("Source entity must have ObservedBy");

        world
            .entity_mut(target)
            .insert(ObservedBy(observed_by.clone()));

        for observer_entity in observed_by.iter().copied() {
            let mut observer_state = world
                .get_mut::<Observer>(observer_entity)
                .expect("Source observer entity must have Observer");
            observer_state.descriptor.entities.push(target);
            let event_keys = observer_state.descriptor.events.clone();
            let components = observer_state.descriptor.components.clone();
            for event_key in event_keys {
                let observers = world.observers.get_observers_mut(event_key);
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
                        if let Some(map) =
                            observers.entity_component_observers.get(&source).cloned()
                        {
                            observers.entity_component_observers.insert(target, map);
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
        entity::EntityCloner,
        event::{EntityEvent, Event},
        observer::On,
        resource::Resource,
        system::ResMut,
        world::World,
    };

    #[derive(Resource, Default)]
    struct Num(usize);

    #[derive(Event, EntityEvent)]
    struct E;

    #[test]
    fn clone_entity_with_observer() {
        let mut world = World::default();
        world.init_resource::<Num>();

        let e = world
            .spawn_empty()
            .observe(|_: On<E>, mut res: ResMut<Num>| res.0 += 1)
            .id();
        world.flush();

        world.trigger_targets(E, e);

        let e_clone = world.spawn_empty().id();
        EntityCloner::build_opt_out(&mut world)
            .add_observers(true)
            .clone_entity(e, e_clone);

        world.trigger_targets(E, [e, e_clone]);

        assert_eq!(world.resource::<Num>().0, 3);
    }
}

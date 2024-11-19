//! Utilities for automatically updating components in response to other ECS data changing.

use crate as bevy_ecs;
use bevy_ecs::{
    component::{ComponentHooks, StorageType, Tick},
    prelude::*,
};
use bevy_ecs_macros::Resource;
use bevy_utils::HashMap;
use core::any::TypeId;

/// TODO: Docs.
pub struct ReactiveComponent<Input: Component, Output: Component> {
    source: Entity,
    expression: Option<Box<dyn (Fn(&Input) -> Output) + Send + Sync + 'static>>,
}

impl<Input: Component, Output: Component> ReactiveComponent<Input, Output> {
    /// TODO: Docs.
    pub fn new(
        source: Entity,
        expression: impl (Fn(&Input) -> Output) + Send + Sync + 'static,
    ) -> Self {
        Self {
            source,
            expression: Some(Box::new(expression)),
        }
    }

    fn on_add_hook(entity: Entity, world: &mut World) {
        let (source, expression) = {
            let mut entity = world.entity_mut(entity);
            let mut this = entity.get_mut::<Self>().unwrap();
            (this.source, this.expression.take().unwrap())
        };

        // Compute and insert initial output
        let input = world
            .get_entity(source)
            .expect("TODO: Source entity despawned")
            .get::<Input>()
            .expect("TODO: Source component removed");
        let output = (expression)(input);
        world.entity_mut(entity).insert(output);

        // Register the subscription
        let subscription = move |world: &mut World, last_run, this_run| {
            let mut input = world
                .get_entity(source)
                .expect("TODO: Source entity despawned")
                .get_ref::<Input>()
                .expect("TODO: Source component removed");
            input.ticks.last_run = last_run;
            input.ticks.this_run = this_run;

            let changed = input.is_changed();
            if changed {
                let output = (expression)(&input);
                world.entity_mut(entity).insert(output);
            }
            changed
        };
        world
            .resource_mut::<ReactiveComponentExpressions>()
            .0
            .insert((entity, TypeId::of::<Self>()), Box::new(subscription));
    }

    fn on_remove_hook(entity: Entity, world: &mut World) {
        // Deregister the subscription
        world
            .resource_mut::<ReactiveComponentExpressions>()
            .0
            .remove(&(entity, TypeId::of::<Self>()));

        // Remove the computed output
        if let Ok(mut entity) = world.get_entity_mut(entity) {
            entity.remove::<Output>();
        }
    }
}

impl<Input: Component, Output: Component> Component for ReactiveComponent<Input, Output> {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks
            .on_add(|mut world, entity, _| {
                world
                    .commands()
                    .queue(move |world: &mut World| Self::on_add_hook(entity, world));
            })
            .on_remove(|mut world, entity, _| {
                world
                    .commands()
                    .queue(move |world: &mut World| Self::on_remove_hook(entity, world));
            });
    }
}

/// System to check for changes to [`ReactiveComponent`] expressions and if changed, recompute it.
pub fn update_reactive_components(world: &mut World) {
    world.resource_scope(
        |world, expressions: Mut<ReactiveComponentExpressions>| loop {
            let last_run = world.last_change_tick();
            let this_run = world.change_tick();
            let mut any_reaction = false;

            for expression in expressions.0.values() {
                any_reaction = any_reaction || (expression)(world, last_run, this_run);
            }

            if !any_reaction {
                break;
            } else {
                world.increment_change_tick();
            }
        },
    );
}

/// TODO: Docs.
#[derive(Resource, Default)]
pub struct ReactiveComponentExpressions(
    HashMap<
        (Entity, TypeId),
        Box<dyn (Fn(&mut World, Tick, Tick) -> bool) + Send + Sync + 'static>,
    >,
);

#[cfg(test)]
mod tests {
    use crate as bevy_ecs;
    use bevy_ecs::{
        prelude::*,
        reactivity::{update_reactive_components, ReactiveComponentExpressions},
    };

    #[derive(Component)]
    struct Foo(u32);

    #[derive(Component, PartialEq, Eq, Debug)]
    struct Bar(u32);

    #[test]
    fn test_reactive_component() {
        let mut world = World::new();
        world.init_resource::<ReactiveComponentExpressions>();

        let source = world.spawn(Foo(0)).id();
        let sink = world
            .spawn(ReactiveComponent::new(source, |foo: &Foo| Bar(foo.0)))
            .id();

        world.flush();

        assert_eq!(world.entity(sink).get::<Bar>(), Some(&Bar(0)));

        let last_tick = world.increment_change_tick();

        world.get_mut::<Foo>(source).unwrap().0 += 1;

        world.last_change_tick_scope(last_tick, update_reactive_components);

        assert_eq!(world.entity(sink).get::<Bar>(), Some(&Bar(1)));
    }
}

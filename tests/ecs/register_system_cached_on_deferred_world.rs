//! A test to confirm that [`bevy::ecs::world::DeferredWorld::register_system_cached`] is caching systems.
//! This is run in CI.

use bevy::{
    ecs::{lifecycle::HookContext, system::SystemId, world::DeferredWorld},
    prelude::*,
};

fn main() {
    let mut app = App::new();

    let world = app.world_mut();

    let first_entity = world.spawn(ComponentWithOnAddHook).id();
    let second_entity = world.spawn(ComponentWithOnAddHook).id();

    app.update();

    let world = app.world_mut();

    let first_system = world
        .entity(first_entity)
        .get::<RegisteredSystem>()
        .unwrap()
        .0;
    let second_system = world
        .entity(second_entity)
        .get::<RegisteredSystem>()
        .unwrap()
        .0;

    assert_eq!(first_system, second_system);
}

#[derive(Component)]
#[component(on_add = Self::on_add)]
struct ComponentWithOnAddHook;

impl ComponentWithOnAddHook {
    fn on_add(mut world: DeferredWorld, context: HookContext) {
        let system_id = world.register_system_cached(Self::the_system_that_will_be_registered);
        world
            .commands()
            .entity(context.entity)
            .insert(RegisteredSystem(system_id));
    }

    fn the_system_that_will_be_registered() {}
}

#[derive(Component)]
struct RegisteredSystem(SystemId);

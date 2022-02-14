use bevy::{
    ecs::system::{lifetimeless::*, SystemState},
    prelude::*,
};

fn main() {
    App::new()
        .init_resource::<MyCustomSchedule>()
        .insert_resource(5u32)
        .add_system(simple_exclusive_system.exclusive_system())
        .add_system(stateful_exclusive_system.exclusive_system())
        .run();
}

#[derive(Default)]
struct MyCustomSchedule(Schedule);

#[derive(Component)]
struct MyComponent;

/// Just a simple exclusive system - this function will run with mutable access to
/// the main app world. This lets it run other schedules, or modify and query the
/// world in hard-to-predict ways, which makes it a powerful primitive. However, because
/// this is usually not needed, and because such wide access makes parallelism impossible,
/// it should generally be avoided.
fn simple_exclusive_system(world: &mut World) {
    world.resource_scope(|world, mut my_schedule: Mut<MyCustomSchedule>| {
        // The resource_scope method is one of the main tools for working with &mut World.
        // This method will temporarily remove the resource from the ECS world and let you
        // access it while still keeping &mut World. A particularly popular pattern is storing
        // schedules, stages, and other similar "runnables" in the world, taking them out
        // using resource_scope, and running them with the world:
        my_schedule.0.run(world);
        // This is fairly simple, but you can implement rather complex custom executors in this manner.
    });
}

/// While it's usually not recommended due to parallelism concerns, you can also use exclusive systems
/// as mostly-normal systems but with the ability to change parameter sets and flush commands midway through.
fn stateful_exclusive_system(
    world: &mut World,
    mut part_one_state: Local<SystemState<(SRes<u32>, SCommands)>>,
    mut part_two_state: Local<SystemState<SQuery<Read<MyComponent>>>>,
) {
    let (resource, mut commands) = part_one_state.get(world);
    let res = *resource as usize;
    commands.spawn_batch((0..res).map(|_| (MyComponent,)));

    // Don't forget to apply your state, or commands won't take effect!
    part_one_state.apply(world);
    let query = part_two_state.get(world);
    let entity_count = query.iter().len();
    // note how the entities spawned in this system are observed,
    // and how resources fetched in earlier stages can still be
    // used if they're cloned out, or small enough to copy out.
    assert_eq!(entity_count, res);
}

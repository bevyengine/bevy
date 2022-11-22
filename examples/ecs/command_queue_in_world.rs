//! This example shows how to create a command queue and store it in the world.
//! This can be useful for conditionally applying a command queue.

use bevy::{
    ecs::{entity::Entities, system::CommandQueue},
    prelude::*,
};

// new typed command queue
#[derive(Resource, Default)]
struct MyCommandQueue(pub CommandQueue);

// marker component for spawned entity
#[derive(Component)]
struct EntityMarker;

fn main() {
    App::new()
        .init_resource::<MyCommandQueue>()
        .add_system(spawn_something)
        .add_system(apply_queue.at_end())
        .add_system_to_stage(CoreStage::PostUpdate, log_entity)
        .run();
}

fn spawn_something(entities: &Entities, mut q: ResMut<MyCommandQueue>) {
    // a command queue by itself is not very useful, but we can create a Commands from it.
    let mut commands = Commands::from_entities(&mut q.0, entities);
    commands.spawn(EntityMarker);
}

fn apply_queue(world: &mut World) {
    world.resource_scope(|world, mut q: Mut<MyCommandQueue>| {
        // apply the stored command queue to the world
        q.0.apply(world);
    });
}

fn log_entity(query: Query<Entity, With<EntityMarker>>) {
    if let Ok(_entitiy) = query.get_single() {
        println!("found entitiy");
    }
}

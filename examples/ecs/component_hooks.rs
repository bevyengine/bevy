use std::collections::HashSet;

use bevy::prelude::*;

#[derive(Component, Debug)]
struct MyComponent(usize);

#[derive(Resource, Default, Debug, Deref, DerefMut)]
struct MyComponentIndex(HashSet<Entity>);

fn main() {
    App::new()
        .add_systems(Startup, (setup, trigger_hooks).chain())
        .init_resource::<MyComponentIndex>()
        .run();
}

fn setup(world: &mut World) {
    world
        .init_component::<MyComponent>()
        .on_add(|mut world, entity| {
            println!("Added MyComponent to: {:?}", entity);
            world.resource_mut::<MyComponentIndex>().insert(entity);
        })
        .on_remove(|mut world, entity| {
            println!(
                "Removed MyComponent from: {:?} {:?}",
                entity,
                world.get::<MyComponent>(entity)
            );
            let mut index = world.resource_mut::<MyComponentIndex>();
            index.remove(&entity);
            println!("Current index: {:?}", *index)
        });
}

fn trigger_hooks(mut commands: Commands) {
    let entity_a = commands.spawn(MyComponent(0)).id();
    let entity_b = commands.spawn(MyComponent(1)).id();
    commands.entity(entity_b).despawn();
    commands.entity(entity_a).despawn();
}

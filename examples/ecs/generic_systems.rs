use bevy::prelude::*;
use bevy::ecs::component::Component;

// When entities spawn in call an event

struct A;
struct B;

fn spawn_entities<T: Component>(mut commands: Commands, q: Query<Entity, With<T>>) {
    q.for_each(|entity| { commands.entity(entity).spawn();});
    info!("Entity {:?} spawned in!")
}



fn main() {
    App::build()
        .add_system(spawn_entities::<A>.system())
        .add_system(spawn_entities::<B>.system())
        .run();
}
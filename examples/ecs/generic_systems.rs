use bevy::prelude::*;
use bevy::ecs::component::Component;

struct A;
struct B;

pub fn spawn_entities<T: Component>(mut commands: Commands,
    mouse_button_inputs: Res<Input<MouseButton>>,) {
    for _ in 0..10 {
        let id = commands.spawn().insert(T).id();
        if mouse_button_inputs.just_pressed(MouseButton::Left) {
            return info!("Spawned entity {:?} with component {}", id, std::any::type_name::<T>());
        }
        if mouse_button_inputs.just_pressed(MouseButton::Right) {
            return info!("Spawned entity {:?} with component {}", id, std::any::type_name::<T>());
        }
    }
}


fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_system(spawn_entities::<A>.system())
        .add_system(spawn_entities::<B>.system())
        .run();
}
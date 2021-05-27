use bevy::prelude::*;
use bevy::ecs::component::Component;

struct A;
struct B;

fn spawn_entities_on_click<T: Component, const N: usize>(
    mut cmds: Commands,
    mouse_button_inputs: Res<Input<MouseButton>>,
) {
    if mouse_button_inputs.just_pressed(MouseButton::Left) {
        for _ in 0..N {
            cmds.spawn().insert(T).id();
        }
        info!("spawned {} entities with component {}", N, std::any::type_name::<T>());
    }
}


fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_system(spawn_entities_on_click::<A>.system())
        .add_system(spawn_entities_on_click::<B>.system())
        .run();
}
use bevy::{prelude::*, ecs::component::Component};

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_system(spawn_entities_on_click::<A>.system())
        .add_system(spawn_entities_on_click::<B>.system())
        .run();
}

struct A;
struct B;

fn spawn_entities_on_click<T: Component, const N: usize>(
    mut cmds: Commands,
    mouse_button_inputs: Res<Input<MouseButton>>,
)
where
    T: Component + Default
    {
    if mouse_button_inputs.just_pressed(MouseButton::Left) {
        for _ in 0..N {
            cmds.spawn().insert(T::default()).id();
        }
        info!("spawned {} entities with component {}", N, std::any::type_name::<T>());
    }
}


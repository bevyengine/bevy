use bevy::prelude::*;
use bevy::winit::WinitPlugin;

fn main() {
    App::new()
        .add_plugins_with(DefaultPlugins, |group| group.disable::<WinitPlugin>())
        .add_system(setup_system)
        .run();
}

fn setup_system(mut commands: Commands) {
    commands.spawn_bundle(PerspectiveCameraBundle::default());
}

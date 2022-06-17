//! Create an application without winit (runs single time, no event loop).

use bevy::prelude::*;
use bevy::winit::WinitPlugin;

#[bevy_main]
async fn main() {
    App::new()
        .add_plugins_with(DefaultPlugins, |group| group.disable::<WinitPlugin>())
        .await
        .add_system(setup_system)
        .run();
}

fn setup_system(mut commands: Commands) {
    commands.spawn_bundle(Camera3dBundle::default());
}

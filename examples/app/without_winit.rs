//! Create an application without winit (runs single time, no event loop).

use bevy::prelude::*;
use bevy::winit::WinitPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.build().disable::<WinitPlugin>())
        .add_systems(Update, setup_system)
        .run();
}

fn setup_system(mut commands: Commands) {
    commands.spawn(Camera3dBundle::default());
}

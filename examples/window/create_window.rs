//! Shows how to create and manipulate windows in bevy using WindowCommands
//! 

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .run();
}

// TODO: Make example

fn window_setup(mut commands: Commands) {
    commands.window();
}
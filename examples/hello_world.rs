//! A minimal example that outputs "hello world"

use bevy::prelude::*;

fn main() {
    App::new().add_systems(Startup, hello_world_system).run();
}

fn hello_world_system() {
    info!("Hello, World!");
}

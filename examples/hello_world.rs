//! A minimal example that outputs "hello world"

use bevy::prelude::*;

fn main() -> AppExit {
    App::new().add_systems(Startup, hello_world_system).run()
}

fn hello_world_system() {
    println!("hello world");
}

use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_system(hello_world_system.into_system("hello"))
        .run();
}

pub fn hello_world_system() {
    println!("hello world");
}
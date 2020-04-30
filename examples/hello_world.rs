use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_system(hello_world.system())
        .run();
}

pub fn hello_world() {
    println!("hello world");
}
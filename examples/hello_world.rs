use bevy::prelude::*;

fn main() {
    App::build().add_system(hello_world_system.system()).run();
}

fn hello_world_system() {
    println!("hello world");
}

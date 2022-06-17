use bevy::prelude::*;

#[bevy_main]
async fn main() {
    App::new().add_system(hello_world_system).run();
}

fn hello_world_system() {
    println!("hello world");
}

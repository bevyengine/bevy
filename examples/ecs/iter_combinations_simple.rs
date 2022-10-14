//! A simple example showing how iter_combinations works

use bevy::prelude::*;

fn main() {
    // App::new().add_system(hello_world).run();
}

#[derive(Component)]
struct A(usize);

fn add_entities(mut commands: Commands) {
    commands.spawn();
}

// fn example(query: Query<&A>) {
// }

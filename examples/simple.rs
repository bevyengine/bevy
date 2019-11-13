use legion::prelude::*;
use bevy::{Application, Transform};


struct SimpleApp;

impl Application for SimpleApp {
    fn update(&self) {}
}

fn main() {
    let app = SimpleApp {};
    // Create a world to store our entities
    let universe = Universe::new();
    let mut world = universe.create_world();
    world.insert((), vec![(Transform::new(),)]);
    app.start();
}

use bevy::{Application};
use legion::prelude::*;

fn main() {
    // Create a world to store our entities
    let universe = Universe::new();
    let mut world = universe.create_world();
    Application::run(universe, world);
}

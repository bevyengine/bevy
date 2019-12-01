use bevy::*;
use bevy::legion::prelude::*;

fn main() {
    let universe = Universe::new();
    let mut world = universe.create_world();
    world.insert((), vec![(Transform::new(),)]);

    // Create a query which finds all `Position` and `Velocity` components
    // let mut query = Read::<Transform>::query();
    Application::run();
}
use bevy::{Application, Transform};
use legion::prelude::*;

struct SimpleApp;

fn main() {
    Application::run();
    // Create a world to store our entities
    let universe = Universe::new();
    let mut world = universe.create_world();
    world.insert((), vec![(Transform::new(),)]);

    // Create a query which finds all `Position` and `Velocity` components
    let mut query = Read::<Transform>::query();
    

    // // Iterate through all entities that match the query in the world
    for mut trans in query.iter(&mut world) {
        // println!("{} hi", trans.global);
    }
}

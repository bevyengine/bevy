use bevy_ecs::prelude::*;
use bevy_entropy::Entropy;
use rand::{prelude::SmallRng, Rng, SeedableRng};
use std::ops::Deref;

// In this example we add a counter resource and increase it's value in one system,
// while a different system prints the current count to the console.
fn main() {
    // Create a world
    let mut world = World::new();

    // Add the entropy resource
    let world_seed = [1; 32];
    world.insert_resource(Entropy::from(world_seed));

    // Add the counter resource
    world.insert_resource(Counter { value: 0 });

    // Create a schedule and a stage
    let mut schedule = Schedule::default();
    let mut update = SystemStage::parallel();

    // Add systems to increase the counter and to print out the current value
    update.add_system(increase_counter);
    update.add_system(print_counter.after(increase_counter));
    schedule.add_stage("update", update);

    for iteration in 1..=10 {
        println!("Simulating frame {}/10", iteration);
        schedule.run(&mut world);
    }
}

// Counter resource to be increased and read by systems
#[derive(Debug)]
struct Counter {
    pub value: i32,
}

fn increase_counter(mut counter: ResMut<Counter>, mut entropy: ResMut<Entropy>) {
    // Note that in a real system it would be better to create this once
    // as a resource.
    let mut rng = SmallRng::from_seed(entropy.get());
    if rng.gen_bool(0.5) {
        counter.value += 1;
        println!("    Increased counter value");
    }
}

fn print_counter(counter: Res<Counter>) {
    println!("    {:?}", counter.deref());
}

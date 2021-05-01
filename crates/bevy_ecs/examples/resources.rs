use bevy_ecs::prelude::*;
use bevy_ecs::system::Command;
use std::ops::Deref;

// This is our resource
#[derive(Debug)]
struct Counter {
    pub value: i32,
}

fn main() {
    // Create a world
    let mut world = World::new();

    // Add the counter resource
    world.insert_resource(Counter { value: 0 });

    // Create a schedule and a stage
    let mut schedule = Schedule::default();
    let mut update = SystemStage::parallel();

    // Add systems sending and receiving events
    update.add_system(increase_counter.system().label("increase"));
    update.add_system(print_counter.system().after("increase"));
    schedule.add_stage("update", update);

    for iteration in 1..=10 {
        println!("Simulating frame {}/10", iteration);
        schedule.run_once(&mut world);
    }
}

fn increase_counter(mut counter: ResMut<Counter>) {
    let random_value: f32 = rand::random();
    if random_value > 0.5 {
        counter.value += 1;
        println!("    Increased counter value");
    }
}

fn print_counter(counter: Res<Counter>) {
    println!("    {:?}", counter.deref());
}

use bevy_ecs::event::Events;
use bevy_ecs::prelude::*;

// This is our event
#[derive(Debug)]
struct MyEvent {
    pub message: String,
    pub random_value: f32,
}

fn main() {
    // Create a world and add the event as a resource
    let mut world = World::new();
    world.insert_resource(Events::<MyEvent>::default());

    // Create a schedule and a stage
    let mut schedule = Schedule::default();
    let mut update = SystemStage::parallel();

    // Add the event managing system to the stage
    update.add_system(Events::<MyEvent>::update_system.system());

    // Add systems sending and receiving events
    update.add_system(sending_system.system());
    update.add_system(receiving_system.system());

    schedule.add_stage("update", update);

    for iteration in 1..=10 {
        println!("Simulating frame {}/10", iteration);
        schedule.run_once(&mut world);
    }
}

fn sending_system(mut event_writer: EventWriter<MyEvent>) {
    let random_value: f32 = rand::random();
    if random_value > 0.5 {
        event_writer.send(MyEvent {
            message: "A random event with value > 0.5".to_string(),
            random_value,
        });
    }
}

fn receiving_system(mut event_reader: EventReader<MyEvent>) {
    for my_event in event_reader.iter() {
        println!("    Received: {:?}", *my_event);
    }
}

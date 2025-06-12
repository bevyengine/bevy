//! In this example a system sends a custom buffered event with a 50/50 chance during any frame.
//! If an event was sent, it will be printed by the console in a receiving system.

#![expect(clippy::print_stdout, reason = "Allowed in examples.")]

use bevy_ecs::{event::EventRegistry, prelude::*};

fn main() {
    // Create a new empty world and add the event as a resource
    let mut world = World::new();
    // The event registry is stored as a resource, and allows us to quickly update all events at once.
    // This call adds both the registry resource and the events resource into the world.
    EventRegistry::register_event::<MyEvent>(&mut world);

    // Create a schedule to store our systems
    let mut schedule = Schedule::default();

    // Buffered events need to be updated every frame in order to clear our buffers.
    // This update should happen before we use the events.
    // Here, we use system sets to control the ordering.
    #[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
    pub struct EventFlusherSystems;

    schedule.add_systems(bevy_ecs::event::event_update_system.in_set(EventFlusherSystems));

    // Add systems sending and receiving events after the events are flushed.
    schedule.add_systems((
        sending_system.after(EventFlusherSystems),
        receiving_system.after(sending_system),
    ));

    // Simulate 10 frames of our world
    for iteration in 1..=10 {
        println!("Simulating frame {iteration}/10");
        schedule.run(&mut world);
    }
}

// This is our event that we will send and receive in systems
#[derive(BufferedEvent)]
struct MyEvent {
    pub message: String,
    pub random_value: f32,
}

// In every frame we will send an event with a 50/50 chance
fn sending_system(mut event_writer: EventWriter<MyEvent>) {
    let random_value: f32 = rand::random();
    if random_value > 0.5 {
        event_writer.write(MyEvent {
            message: "A random event with value > 0.5".to_string(),
            random_value,
        });
    }
}

// This system listens for events of the type MyEvent
// If an event is received it will be printed to the console
fn receiving_system(mut event_reader: EventReader<MyEvent>) {
    for my_event in event_reader.read() {
        println!(
            "    Received message {}, with random value of {}",
            my_event.message, my_event.random_value
        );
    }
}

//! In this example a system sends a custom messages with a 50/50 chance during any frame.
//! If a message was sent, it will be printed by the console in a receiving system.

#![expect(clippy::print_stdout, reason = "Allowed in examples.")]

use bevy_ecs::{message::MessageRegistry, prelude::*};

fn main() {
    // Create a new empty world.
    let mut world = World::new();
    // The message registry is stored as a resource, and allows us to quickly update all messages at once.
    // This call adds both the registry resource and the `Messages` resource into the world.
    MessageRegistry::register_message::<MyMessage>(&mut world);

    // Create a schedule to store our systems
    let mut schedule = Schedule::default();

    // Messages need to be updated every frame in order to clear our buffers.
    // This update should happen before we use the messages.
    // Here, we use system sets to control the ordering.
    #[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
    pub struct EventFlusherSystems;

    schedule.add_systems(bevy_ecs::message::message_update_system.in_set(EventFlusherSystems));

    // Add systems sending and receiving messages after the messages are flushed.
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

// This is our message that we will send and receive in systems
#[derive(Message)]
struct MyMessage {
    pub message: String,
    pub random_value: f32,
}

// In every frame we will send a message with a 50/50 chance
fn sending_system(mut message_writer: MessageWriter<MyMessage>) {
    let random_value: f32 = rand::random();
    if random_value > 0.5 {
        message_writer.write(MyMessage {
            message: "A random message with value > 0.5".to_string(),
            random_value,
        });
    }
}

// This system listens for messages of the type MyEvent
// If a message is received it will be printed to the console
fn receiving_system(mut message_reader: MessageReader<MyMessage>) {
    for my_message in message_reader.read() {
        println!(
            "    Received message {}, with random value of {}",
            my_message.message, my_message.random_value
        );
    }
}

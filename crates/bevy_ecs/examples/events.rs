use bevy_ecs::{event::Events, prelude::*};

// In this example a system sends a custom event with a 50/50 chance during any frame.
// If an event was send, it will be printed by the console in a receiving system.
fn main() {
    // Create a new empty world and add the event as a resource
    let mut world = World::new();
    world.insert_resource(Events::<MyEvent>::default());

    // Create a schedule and a stage
    let mut schedule = Schedule::default();

    // Events need to be updated in every frame. This update should happen before we use
    // the events. To guarantee this, we can let the update run in an earlier stage than our logic.
    // Here we will use a stage called "first" that will always run it's systems before the Stage
    // called "second". In "first" we update the events and in "second" we run our systems
    // sending and receiving events.
    let mut first = SystemStage::parallel();
    first.add_system(Events::<MyEvent>::update_system.system());
    schedule.add_stage("first", first);

    // Add systems sending and receiving events to a "second" Stage
    let mut second = SystemStage::parallel();
    second.add_system(sending_system.system().label(EventSystem::Sending));
    second.add_system(receiving_system.system().after(EventSystem::Sending));

    // Run the "second" Stage after the "first" Stage, so our Events always get updated before we use them
    schedule.add_stage_after("first", "second", second);

    // Simulate 10 frames of our world
    for iteration in 1..=10 {
        println!("Simulating frame {}/10", iteration);
        schedule.run(&mut world);
    }
}

// System label to enforce a run order of our systems
#[derive(SystemLabel, Debug, Clone, PartialEq, Eq, Hash)]
enum EventSystem {
    Sending,
}

// This is our event that we will send and receive in systems
#[derive(Debug)]
struct MyEvent {
    pub message: String,
    pub random_value: f32,
}

// In every frame we will send an event with a 50/50 chance
fn sending_system(mut event_writer: EventWriter<MyEvent>) {
    let random_value: f32 = rand::random();
    if random_value > 0.5 {
        event_writer.send(MyEvent {
            message: "A random event with value > 0.5".to_string(),
            random_value,
        });
    }
}

// This system listens for events of the type MyEvent
// If an event is received it will be printed to the console
fn receiving_system(mut event_reader: EventReader<MyEvent>) {
    for my_event in event_reader.iter() {
        println!("    Received: {:?}", *my_event);
    }
}

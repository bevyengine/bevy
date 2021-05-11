use bevy::core::FixedTimestep;
use bevy::prelude::*;

/// This example creates a new event, a system that triggers the event once per second,
/// and a system that prints a message whenever the event is received.
fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_event::<MyEvent>()
        .add_system(
            event_trigger_system
                .system()
                .with_run_criteria(FixedTimestep::step(1.0)),
        )
        .add_system(event_listener_system.system())
        .run();
}

struct MyEvent {
    pub message: String,
}

// sends MyEvent every second
fn event_trigger_system(mut my_events: EventWriter<MyEvent>) {
    my_events.send(MyEvent {
        message: "MyEvent just happened!".to_string(),
    });
}

// prints events as they come in
fn event_listener_system(mut events: EventReader<MyEvent>) {
    for my_event in events.iter() {
        info!("{}", my_event.message);
    }
}

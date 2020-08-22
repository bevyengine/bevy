use bevy::prelude::*;

/// This example creates a new event, a system that triggers the event once per second,
/// and a system that prints a message whenever the event is received.
fn main() {
    App::build()
        .add_default_plugins()
        .add_event::<MyEvent>()
        .init_resource::<EventTriggerState>()
        .init_resource::<EventListenerState>()
        .add_system(event_trigger_system.system())
        .add_system(event_listener_system.system())
        .run();
}

struct MyEvent {
    pub message: String,
}

struct EventTriggerState {
    event_timer: Timer,
}

impl Default for EventTriggerState {
    fn default() -> Self {
        EventTriggerState {
            event_timer: Timer::from_seconds(1.0, true),
        }
    }
}

// sends MyEvent every second
fn event_trigger_system(
    time: Res<Time>,
    mut state: ResMut<EventTriggerState>,
    mut my_events: ResMut<Events<MyEvent>>,
) {
    state.event_timer.tick(time.delta_seconds);
    if state.event_timer.finished {
        my_events.send(MyEvent {
            message: "MyEvent just happened!".to_string(),
        });
    }
}

#[derive(Default)]
struct EventListenerState {
    my_event_reader: EventReader<MyEvent>,
}

// prints events as they come in
fn event_listener_system(mut state: ResMut<EventListenerState>, my_events: Res<Events<MyEvent>>) {
    for my_event in state.my_event_reader.iter(&my_events) {
        println!("{}", my_event.message);
    }
}

use bevy::prelude::*;

/// This example creates a new event, a system that triggers the event once per second,
/// and a system that prints a message whenever the event is received.
fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_event::<MyEvent>()
        .init_resource::<EventTriggerState>()
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
    if state.event_timer.tick(time.delta_seconds()).finished() {
        my_events.send(MyEvent {
            message: "MyEvent just happened!".to_string(),
        });
    }
}

// prints events as they come in
fn event_listener_system(
    mut my_event_reader: Local<EventReader<MyEvent>>,
    my_events: Res<Events<MyEvent>>,
) {
    for my_event in my_event_reader.iter(&my_events) {
        println!("{}", my_event.message);
    }
}

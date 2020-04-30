use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_event::<MyEvent>()
        .add_resource(EventTriggerState::default())
        .add_resource_init::<EventListenerState>()
        .add_system(event_trigger_system.into_system("event_trigger"))
        .add_system(event_listener_system.into_system("event_listener"))
        .run();
}

struct MyEvent {
    pub message: String,
}

#[derive(Default)]
struct EventTriggerState {
    elapsed: f32,
}

// sends MyEvent every second
fn event_trigger_system(
    mut state: ResourceMut<EventTriggerState>,
    mut my_events: ResourceMut<Events<MyEvent>>,
    time: Resource<Time>,
) {
    state.elapsed += time.delta_seconds;
    if state.elapsed > 1.0 {
        my_events.send(MyEvent {
            message: "Hello World".to_string(),
        });

        state.elapsed = 0.0;
    }
}

struct EventListenerState {
    my_event_reader: EventReader<MyEvent>,
}

impl From<&mut Resources> for EventListenerState {
    fn from(resources: &mut Resources) -> Self {
        EventListenerState {
            my_event_reader: resources.get_event_reader::<MyEvent>(),
        }
    }
}

// prints events as they come in
fn event_listener_system(
    mut state: ResourceMut<EventListenerState>,
    my_events: Resource<Events<MyEvent>>,
) {
    for my_event in state.my_event_reader.iter(&my_events) {
        println!("{}", my_event.message);
    }
}

use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_event::<MyEvent>()
        .add_resource(EventTriggerState::default())
        .add_resource_init::<EventListenerState>()
        .add_system(into_resource_system("event_trigger", event_trigger_system))
        .add_system(into_resource_system(
            "event_listener",
            event_listener_system,
        ))
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
    (state, my_events, time): &mut (
        ResourceMut<EventTriggerState>,
        ResourceMut<Events<MyEvent>>,
        Resource<Time>,
    ),
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
    (state, my_events): &mut (ResourceMut<EventListenerState>, Resource<Events<MyEvent>>),
) {
    for my_event in my_events.iter(&mut state.my_event_reader) {
        println!("{}", my_event.message);
    }
}
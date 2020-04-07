use bevy::prelude::*;

struct MyEvent {
    pub message: String,
}

fn main() {
    App::build()
        .add_default_plugins()
        .add_event::<MyEvent>()
        .add_system(event_trigger_system())
        .build_system(event_listener_system)
        .run();
}

// sends MyEvent every second
fn event_trigger_system() -> Box<dyn Schedulable> {
    let mut elapsed = 0.0;
    SystemBuilder::new("event_trigger")
        .read_resource::<Time>()
        .write_resource::<Events<MyEvent>>()
        .build(move |_, _, (time, my_events), _| {
            elapsed += time.delta_seconds;
            if elapsed > 1.0 {
                my_events.send(MyEvent {
                    message: "Hello World".to_string(),
                });

                elapsed = 0.0;
            }
        })
}

// prints events as they come in
fn event_listener_system(resources: &mut Resources) -> Box<dyn Schedulable> {
    let mut my_event_reader = resources.get_event_reader::<MyEvent>();
    SystemBuilder::new("event_listener")
        .read_resource::<Events<MyEvent>>()
        .build(move |_, _, my_events, _| {
            for my_event in my_events.iter(&mut my_event_reader) {
                println!("{}", my_event.message);
            }
        })
}

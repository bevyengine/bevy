use bevy::prelude::*;

struct MyEvent {
    pub message: String,
}

fn main() {
    App::build()
        .add_defaults()
        .add_event::<MyEvent>()
        .add_system(event_trigger_system())
        .build_system(event_listener_system)
        .run();
}

fn event_trigger_system() -> Box<dyn Schedulable> {
    let mut elapsed = 0.0;
    SystemBuilder::new("event_trigger")
        .read_resource::<Time>()
        .write_resource::<Event<MyEvent>>()
        .build(move |_, _, (time, my_event), _| {
            elapsed += time.delta_seconds;
            if elapsed > 1.0 {
                my_event.send(MyEvent {
                    message: "Hello World".to_string(),
                });

                elapsed = 0.0;
            }
        })
}

fn event_listener_system(resources: &mut Resources) -> Box<dyn Schedulable> {
    let mut my_event_handle = resources.get_event_handle::<MyEvent>();
    SystemBuilder::new("event_listener")
        .read_resource::<Event<MyEvent>>()
        .build(move |_, _, my_events, _| {
            for my_event in my_events.iter(&mut my_event_handle) {
                println!("{}", my_event.message);
            }
        })
}

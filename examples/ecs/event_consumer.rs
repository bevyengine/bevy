use bevy::prelude::*;
use bevy::{app::Events, core::FixedTimestep};

/// Events are automatically cleaned up after two frames when intialized via `.add_event`.
/// To bypass this, you can simply add the Events::<T> resource manually.
/// This is critical when working with systems that read events but do not run every tick,
/// such as those that operate with a FixedTimeStep run criteria.
///
/// When you do so though, you need to be careful to clean up these events eventually,
/// otherwise the size of your vector of events will grow in an unbounded fashion.
///
/// `EventConsumer::<T>` provides a simple interface to do so, clearing all events that it reads
/// by draining them into a new vector.
/// You can combine it with other `EventReader`s as long as they read events before,
/// but only one `EventConsumer` system should be used per event type in most cases
/// as they will compete for events.
fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        // We can't use .add_event or our events will be cleaned up too soon
        .init_resource::<Events<MyEvent>>()
        .add_system(
            event_trigger_system
                .system()
                .with_run_criteria(FixedTimestep::step(1.0)),
        )
        .add_system(event_listener_system.system().label("listening"))
        .add_system(
            event_devourer_system
                .system()
                // Must occur after event_listener_system or some events may be missed
                .after("listening")
                .with_run_criteria(FixedTimestep::step(5.0)),
        )
        .run();
}

struct MyEvent {
    pub message: String,
}

// sends MyEvent every second
fn event_trigger_system(time: Res<Time>, mut my_events: EventWriter<MyEvent>) {
    my_events.send(MyEvent {
        message: format!(
            "This event was sent at {} milliseconds",
            time.time_since_startup().as_millis()
        ),
    });
}

// reads events as soon as they come in
fn event_listener_system(mut events: EventReader<MyEvent>) {
    for _ in events.iter() {
        info!("I heard an event!");
    }
}

// reports events once every 5 seconds
fn event_devourer_system(events: EventConsumer<MyEvent>) {
    // Events are only consumed when .drain() is called
    for my_event in events.drain() {
        info!("{}", my_event.message);
    }
}

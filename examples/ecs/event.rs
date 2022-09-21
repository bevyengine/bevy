//! This example creates a new event, a system that triggers the event once per second,
//! and a system that prints a message whenever the event is received.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_event::<MyEvent>()
        .add_event::<PlaySound>()
        .init_resource::<EventTriggerState>()
        .add_system(event_trigger)
        .add_system(event_listener)
        .add_system(sound_player)
        .run();
}

struct MyEvent {
    pub message: String,
}

#[derive(Default)]
struct PlaySound;

#[derive(Resource)]
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

// sends MyEvent and PlaySound every second
fn event_trigger(
    time: Res<Time>,
    mut state: ResMut<EventTriggerState>,
    mut my_events: EventWriter<MyEvent>,
    mut play_sound_events: EventWriter<PlaySound>,
) {
    if state.event_timer.tick(time.delta()).finished() {
        my_events.send(MyEvent {
            message: "MyEvent just happened!".to_string(),
        });
        play_sound_events.send_default();
    }
}

// prints events as they come in
fn event_listener(mut events: EventReader<MyEvent>) {
    for my_event in events.iter() {
        info!("{}", my_event.message);
    }
}

fn sound_player(mut play_sound_events: EventReader<PlaySound>) {
    for _ in play_sound_events.iter() {
        info!("Playing a sound");
    }
}

use bevy::{
    input::touch::{TouchFingerInput, TouchMotion},
    prelude::*,
};

fn main() {
    App::build()
        .add_default_plugins()
        .init_resource::<State>()
        .add_system(print_touch_events_system.system())
        .run();
}

#[derive(Default)]
struct State {
    touch_finger_event_reader: EventReader<TouchFingerInput>,
    touch_motion_event_reader: EventReader<TouchMotion>,
}

/// This system prints out all touch events as they come in
fn print_touch_events_system(
    mut state: ResMut<State>,
    touch_finger_input_events: Res<Events<TouchFingerInput>>,
    touch_motion_events: Res<Events<TouchMotion>>,
) {
    for event in state
        .touch_finger_event_reader
        .iter(&touch_finger_input_events)
    {
        println!("{:?}", event);
    }

    for event in state.touch_motion_event_reader.iter(&touch_motion_events) {
        println!("{:?}", event);
    }
}

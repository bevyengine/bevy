use bevy::{
    input::mouse::{MouseButtonInput, MouseMotionInput},
    prelude::*,
};

fn main() {
    App::build()
        .add_default_plugins()
        .add_resource_init::<State>()
        .add_system(mouse_input_system.system())
        .run();
}

#[derive(Resource)]
struct State {
    mouse_button_event_reader: EventReader<MouseButtonInput>,
    mouse_motion_event_reader: EventReader<MouseMotionInput>,
}

/// prints out mouse events as they come in
fn mouse_input_system(
    mut state: ResourceMut<State>,
    mouse_button_input_events: Ref<Events<MouseButtonInput>>,
    mouse_motion_events: Ref<Events<MouseMotionInput>>,
) {
    for event in state
        .mouse_button_event_reader
        .iter(&mouse_button_input_events)
    {
        println!("{:?}", event);
    }

    for event in state.mouse_motion_event_reader.iter(&mouse_motion_events) {
        println!("{:?}", event);
    }
}

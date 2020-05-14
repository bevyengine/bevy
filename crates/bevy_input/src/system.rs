use crate::keyboard::{ElementState, KeyboardInput, VirtualKeyCode};
use bevy_app::{AppExit, Events, GetEventReader};
use legion::prelude::*;

pub fn exit_on_esc_system(resources: &mut Resources) -> Box<dyn Schedulable> {
    let mut keyboard_input_event_reader = resources.get_event_reader::<KeyboardInput>();
    (move |keyboard_input_events: Res<Events<KeyboardInput>>,
           mut app_exit_events: ResMut<Events<AppExit>>| {
        for event in keyboard_input_event_reader.iter(&keyboard_input_events) {
            if let Some(virtual_key_code) = event.virtual_key_code {
                if event.state == ElementState::Pressed
                    && virtual_key_code == VirtualKeyCode::Escape
                {
                    app_exit_events.send(AppExit);
                }
            }
        }
    })
    .system_named("exit_on_esc")
}

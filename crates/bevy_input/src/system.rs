use crate::keyboard::{ElementState, KeyboardInput, VirtualKeyCode};
use bevy_app::{AppExit, Events, GetEventReader};
use legion::prelude::*;

pub fn exit_on_esc_system(resources: &mut Resources) -> Box<dyn Schedulable> {
    let mut keyboard_input_event_reader = resources.get_event_reader::<KeyboardInput>();
    SystemBuilder::new("exit_on_esc")
        .read_resource::<Events<KeyboardInput>>()
        .write_resource::<Events<AppExit>>()
        .build(
            move |_, _, (ref keyboard_input_events, ref mut app_exit_events), _| {
                for event in keyboard_input_events.iter(&mut keyboard_input_event_reader) {
                    if let Some(virtual_key_code) = event.virtual_key_code {
                        if event.state == ElementState::Pressed
                            && virtual_key_code == VirtualKeyCode::Escape
                        {
                            app_exit_events.send(AppExit);
                        }
                    }
                }
            },
        )
}

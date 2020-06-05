use crate::keyboard::{ElementState, KeyboardInput, KeyCode};
use bevy_app::{AppExit, EventReader, Events};
use legion::prelude::*;

pub fn exit_on_esc_system(_resources: &mut Resources) -> Box<dyn Schedulable> {
    let mut keyboard_input_event_reader = EventReader::<KeyboardInput>::default();
    (move |keyboard_input_events: Res<Events<KeyboardInput>>,
           mut app_exit_events: ResMut<Events<AppExit>>| {
        for event in keyboard_input_event_reader.iter(&keyboard_input_events) {
            if let Some(key_code) = event.key_code {
                if event.state == ElementState::Pressed
                    && key_code == KeyCode::Escape
                {
                    app_exit_events.send(AppExit);
                }
            }
        }
    })
    .system_named("exit_on_esc")
}

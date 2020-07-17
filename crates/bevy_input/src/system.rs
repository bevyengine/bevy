use crate::keyboard::{ElementState, KeyCode, KeyboardInput};
use bevy_app::{
    prelude::{EventReader, Events},
    AppExit,
};
use bevy_ecs::{Local, Res, ResMut};

#[derive(Default)]
pub struct ExitOnEscapeState {
    reader: EventReader<KeyboardInput>,
}

pub fn exit_on_esc_system(
    mut state: Local<ExitOnEscapeState>,
    keyboard_input_events: Res<Events<KeyboardInput>>,
    mut app_exit_events: ResMut<Events<AppExit>>,
) {
    for event in state.reader.iter(&keyboard_input_events) {
        if let Some(key_code) = event.key_code {
            if event.state == ElementState::Pressed && key_code == KeyCode::Escape {
                app_exit_events.send(AppExit);
            }
        }
    }
}

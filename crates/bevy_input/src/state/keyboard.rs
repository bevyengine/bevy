use crate::events::KeyboardEvent;
use bevy_app::EventReader;

/// State used by the keyboard input system
#[derive(Default)]
pub struct KeyboardInputState {
    keyboard_input_event_reader: EventReader<KeyboardEvent>,
}

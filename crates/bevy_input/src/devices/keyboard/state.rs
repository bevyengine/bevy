use super::*;
use bevy_app::EventReader;

/// State used by the keyboard input system
#[derive(Default)]
pub struct KeyboardInputState {
    pub(crate) keyboard_input_event_reader: EventReader<KeyboardEvent>,
}

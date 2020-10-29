use super::*;
use bevy_app::EventReader;

/// State used by the mouse button input system
#[derive(Default)]
pub struct MouseButtonInputState {
    pub(crate) mouse_button_input_event_reader: EventReader<MouseButtonEvent>,
}

//! Keyboard input implementation module

use crate::{core::ElementState, keyboard::KeyCode};
use bevy_app::prelude::*;

/// A key input event from a keyboard device
#[derive(Debug, Clone)]
pub struct KeyboardInput {
    pub scan_code: u32,
    pub key_code: Option<KeyCode>,
    pub state: ElementState,
}

/// State used by the keyboard input system
#[derive(Default)]
pub struct KeyboardInputState {
    pub(crate) keyboard_input_event_reader: EventReader<KeyboardInput>,
}

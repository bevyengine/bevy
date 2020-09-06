use bevy_input::{mouse::MouseButton, prelude::KeyCode};
use std::collections::HashMap;

// TODO need to compute deadzone for GetActionStrength()
pub(crate) struct InputMap {
    // action data
    pub(crate) action_raw_strength: HashMap<String, f32>,
    pub(crate) action_deadzone: HashMap<String, f32>,

    // input map
    pub(crate) keyboard_pressed_map: HashMap<KeyCode, String>,
    pub(crate) mouse_pressed_map: HashMap<MouseButton, String>,
}

impl InputMap {
    // actions
    pub fn GetActionStrength(&self, action: String) -> f32 {
        self.action_raw_strength[&action]
    }
    pub fn SetActionStrength(&mut self, action: String, strength: f32) {
        self.action_raw_strength.insert(action, strength);
    }
    pub fn IsActionPressed(&self, action: String) -> bool {
        self.GetActionStrength(action) > 0.0
    }
    pub fn ResetActionStrength(&mut self, action: String) {
        self.SetActionStrength(action, 0.0)
    }
}

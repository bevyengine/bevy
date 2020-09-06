use bevy_ecs::ResMut;
use bevy_input::{mouse::MouseButton, prelude::KeyCode};
use std::collections::HashMap;

// TODO deadzone
#[derive(Default)]
pub struct InputMap {
    // crate
    // action data
    action_raw_strength: HashMap<String, f32>,
    action_deadzone: HashMap<String, f32>,
}

impl InputMap {
    // publics
    pub fn get_action_strength(&self, action: String) -> f32 {
        match self.action_raw_strength.get(&action) {
            Some(n) => return n.clone(),
            None => return 0.,
        }
    }
    pub fn is_action_in_progress(&self, action: String) -> bool {
        self.get_action_strength(action) > 0.0
    }

    pub fn set_dead_zone(&mut self, action: String, value: f32) {
        self.action_deadzone.insert(action, value);
    }

    // crates
    pub(crate) fn set_raw_action_strength(&mut self, action: String, strength: f32) {
        self.action_raw_strength.insert(action, strength);
    }
    pub(crate) fn reset_raw_action_strength(&mut self, action: String) {
        self.set_raw_action_strength(action, 0.0)
    }
    pub(crate) fn reset_all_raw_strength(&mut self) {
        self.action_raw_strength.clear();
    }

    // system
    pub(crate) fn action_reset_system(mut input_map: ResMut<InputMap>) {
        input_map.reset_all_raw_strength();
    }
}

use bevy_input::{mouse::MouseButton, prelude::KeyCode};
use std::collections::HashMap;
use bevy_ecs::ResMut;

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
    pub fn GetActionStrength(&self, action: String) -> f32 {
        
        match self.action_raw_strength.get(&action)
        {
            Some(n) => return  n.clone(),
            None => return 0.
        }
    }
    pub fn IsActionPressed(&self, action: String) -> bool {
        self.GetActionStrength(action) > 0.0
    }

    pub fn SetDeadZone(&mut self, action: String, value: f32)
    {
        self.action_deadzone.insert(action, value);
    }

    // crates
    pub(crate) fn SetRawActionStrength(&mut self, action: String, strength: f32) {
        self.action_raw_strength.insert(action, strength);
    }
    pub(crate) fn ResetRawActionStrength(&mut self, action: String) {
        self.SetRawActionStrength(action, 0.0)
    }
    pub(crate) fn ResetAllRawStrength(&mut self)
    {
        self.action_raw_strength.clear();
    }

    // system
    pub(crate) fn action_reset_system(mut input_map: ResMut<InputMap>)
    {
        input_map.ResetAllRawStrength();
    }
}

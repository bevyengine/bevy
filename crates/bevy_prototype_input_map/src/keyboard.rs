use crate::inputmap::InputMap;
use std::collections::HashMap;
use bevy_ecs::{ Res, ResMut};
use bevy_input::{Input, prelude::KeyCode};


// keyboard
impl InputMap {

    pub fn GetKeyPressedBindingMap(&self) -> &HashMap<KeyCode, String> {
        &self.keyboard_pressed_map
    }

    pub fn BindKeyboardPressed(&mut self, code: KeyCode, action: String, deadzone: f32) {
        self.keyboard_pressed_map.insert(code, action);
    }
    pub fn UnBindKeyboardPressed(&mut self, code: KeyCode, action: String) {
        self.keyboard_pressed_map.remove(&code);
    }

    // system
    pub fn keyboard_input_map_system(key_input: Res<Input<KeyCode>>, mut input_map: ResMut<InputMap>) {
        for (keycode, action) in input_map.GetKeyPressedBindingMap() {
            if key_input.pressed(*keycode) 
            {
                input_map.action_raw_strength.insert(action.clone(), 1.0);
            }
            else
            {
                input_map.action_raw_strength.insert(action.clone(), 0.0);
            }
        }
    }
}

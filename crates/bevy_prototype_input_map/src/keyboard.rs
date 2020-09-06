use crate::inputmap::InputMap;
use bevy_ecs::{Res, ResMut};
use bevy_input::{prelude::KeyCode, Input};
use std::collections::HashMap;

#[derive(Default)]
pub struct KeyboardMap {
    action_binding: HashMap<KeyCode, String>,
}

impl KeyboardMap {
    // publics
    pub fn bind_keyboard_pressed(&mut self, code: KeyCode, action: String) {
        self.action_binding.insert(code, action);
    }

    pub fn unbind_keyboard_pressed(&mut self, code: KeyCode) {
        self.action_binding.remove(&code);
    }

    // system
    pub(crate) fn action_update_system(
        mut input_map: ResMut<InputMap>,
        key_map: Res<KeyboardMap>,
        key_input: Res<Input<KeyCode>>,
    ) {
        let map = &mut input_map;
        let bindings_iter = key_map.action_binding.iter();

        for (keycode, action) in bindings_iter {
            if key_input.pressed(*keycode) {
                map.set_raw_action_strength(action.clone(), 1.0);
            }
        }
    }
}

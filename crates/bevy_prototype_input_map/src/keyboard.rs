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
    pub fn BindKeyboardPressed(&mut self, code: KeyCode, action: String, deadzone: f32) {
        self.action_binding.insert(code, action);
    }
    pub fn UnBindKeyboardPressed(&mut self, code: KeyCode, action: String) {
        self.action_binding.remove(&code);
    }

    // crates
    // pub(crate) fn GetBindings(&mut self) -> &HashMap<KeyCode, String> {
    //     &self.action_binding
    // }

    // system
    pub(crate) fn action_system(
        mut input_map: ResMut<InputMap>,
        key_map: Res<KeyboardMap>,
        key_input: Res<Input<KeyCode>>,
    ) {
        let mut map = &mut input_map;
        let bindings_iter = key_map.action_binding.iter();

        for (keycode, action) in bindings_iter {
            if key_input.pressed(*keycode) {
                map.SetRawActionStrength(action.clone(), 1.0);
            } else {
                map.ResetActionStrength(action.clone());
            }
        }
    }
}

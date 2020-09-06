use crate::inputmap::InputMap;
use crate::axis::Axis;
use bevy_ecs::{Res, ResMut};
use bevy_input::{prelude::MouseButton, Input};
use std::collections::HashMap;

#[derive(Default)]
pub struct MouseMap {
    action_button_binding: HashMap<MouseButton, String>,
    action_move_binding: HashMap<Axis, String>,
}

impl MouseMap {
    // publics
    pub fn BindMousePressed(&mut self, code: MouseButton, action: String) {
        self.action_button_binding.insert(code, action);
    }
    pub fn UnBindMousePressed(&mut self, button: MouseButton) {
        self.action_button_binding.remove(&button);
    }
    pub fn BindMouseMove(&mut self, axis: Axis, action: String, deadzone: f32) {
        self.action_move_binding.insert(axis, action);
    }
    pub fn UnBindMouseMove(&mut self, axis: Axis, action: String) {
        self.action_move_binding.remove(&axis);
    }

    // system
    pub(crate) fn action_update_system(
        mut input_map: ResMut<InputMap>,
        mouse_map: Res<MouseMap>,
        mouse_input: Res<Input<MouseButton>>,
    ) {
        let mut map = &mut input_map;
        let bindings_iter = mouse_map.action_button_binding.iter();

        for (button, action) in bindings_iter {
            if mouse_input.pressed(*button) {
                map.SetRawActionStrength(action.clone(), 1.0);
            }
        }

        // TODO Axis
    }
}

use crate::{axis::Axis, inputmap::InputMap};
use bevy_app::{EventReader, Events};
use bevy_ecs::{Local, Res, ResMut};
use bevy_input::{mouse::MouseMotion, prelude::MouseButton, Input};
use std::collections::HashMap;

// TODO Mouse move

#[derive(Default)]
pub struct MouseMap {
    action_button_binding: HashMap<MouseButton, String>,
    action_move_binding: HashMap<Axis, String>,
}

#[derive(Default)]
pub struct MouseMoveState {
    reader: EventReader<MouseMotion>,
}

impl MouseMap {
    // publics
    pub fn bind_mouse_button_pressed(&mut self, code: MouseButton, action: String) {
        self.action_button_binding.insert(code, action);
    }

    pub fn unbind_mouse_button_pressed(&mut self, button: MouseButton) {
        self.action_button_binding.remove(&button);
    }

    pub fn bind_mouse_motion(&mut self, axis: Axis, action: String) {
        self.action_move_binding.insert(axis, action);
    }

    pub fn unbind_mouse_motion(&mut self, axis: Axis, action: String) {
        self.action_move_binding.remove(&axis);
    }

    // system
    pub(crate) fn button_press_input_system(
        mut input_map: ResMut<InputMap>,
        mouse_map: Res<MouseMap>,
        mouse_button_input: Res<Input<MouseButton>>,
    ) {
        // buttons
        let button_bindings_iter = mouse_map.action_button_binding.iter();
        for (button, action) in button_bindings_iter {
            if mouse_button_input.pressed(*button) {
                input_map.set_raw_action_strength(action.clone(), 1.0);
            }
        }
    }

    pub fn mouse_move_event_system(
        mut input_map: ResMut<InputMap>,
        mouse_map: Res<MouseMap>,

        mut state: Local<MouseMoveState>,
        move_events: Res<Events<MouseMotion>>,
    ) {
        match state.reader.latest(&move_events) {
            Some(value) => {
                let normalised_vec = value.delta.normalize();
                let x = normalised_vec.x();
                let y = normalised_vec.y();

                // horizontal
                if x > 0.0 {
                    match mouse_map.action_move_binding.get(&Axis::X_Positive) {
                        Some(action) => {
                            input_map.set_raw_action_strength(action.clone(), x);
                        }
                        None => {}
                    }
                }

                if x < 0.0 {
                    match mouse_map.action_move_binding.get(&Axis::X_Negative) {
                        Some(action) => {
                            input_map.set_raw_action_strength(action.clone(), x.abs());
                        }
                        None => {}
                    }
                }

                // vertical
                if y > 0.0 {
                    match mouse_map.action_move_binding.get(&Axis::Y_Positive) {
                        Some(action) => {
                            input_map.set_raw_action_strength(action.clone(), y);
                        }
                        None => {}
                    }
                }

                if y < 0.0 {
                    match mouse_map.action_move_binding.get(&Axis::Y_Negative) {
                        Some(action) => {
                            input_map.set_raw_action_strength(action.clone(), y.abs());
                        }
                        None => {}
                    }
                }
            }
            None => {}
        }
    }
}

use std::collections::{HashSet, HashMap};
use bevy_input::{mouse::MouseButton, prelude::KeyCode};

pub struct InputMap
{
    // actions
    action: HashSet<String>,

    // inputs
    keyboard_pressed_map: HashMap<KeyCode, String>,
    keyboard_just_pressed_map: HashMap<KeyCode, String>,
    keyboard_just_released_map: HashMap<KeyCode, String>,
    
    mouse_pressed_map: HashMap<MouseButton, String>,
    mouse_just_pressed_map: HashMap<MouseButton, String>,
    mouse_just_released_map: HashMap<MouseButton, String>,
}


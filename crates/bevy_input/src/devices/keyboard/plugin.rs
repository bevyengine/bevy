//! Keyboard Input Plugin Module.
//! Implements a bevy plugin that adds the resources and systems to facilitate keyboard_device input

use crate::{
    core::input::Input,
    keyboard_devices::{keyboard_input_system, KeyCode, KeyboardInput},
};
use bevy_app::prelude::*;
use bevy_ecs::IntoQuerySystem;

/// Adds keyboard_device input to an App
#[derive(Default)]
pub struct KeyboardInputPlugin;

impl Plugin for KeyboardInputPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_event::<KeyboardInput>()
            .init_resource::<Input<KeyCode>>()
            .add_system_to_stage(bevy_app::stage::EVENT, keyboard_input_system.system());
    }
}

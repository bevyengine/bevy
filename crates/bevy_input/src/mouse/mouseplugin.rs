//! Mouse Input Plugin Module.
//! Implements a bevy plugin that adds the resources and systems to facilitate mouse input

use crate::{
    core::Input,
    mouse::{mouse_button_input_system, MouseButton, MouseButtonInput, MouseMotion, MouseWheel},
};
use bevy_app::prelude::*;
use bevy_ecs::IntoQuerySystem;

/// Adds mouse input to an App
#[derive(Default)]
pub struct MouseInputPlugin;

impl Plugin for MouseInputPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_event::<MouseButtonInput>()
            .add_event::<MouseMotion>()
            .add_event::<MouseWheel>()
            .init_resource::<Input<MouseButton>>()
            .add_system_to_stage(bevy_app::stage::EVENT, mouse_button_input_system.system());
    }
}

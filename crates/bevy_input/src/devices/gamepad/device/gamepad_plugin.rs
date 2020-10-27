//! Gamepad Input Plugin Module.
//! Implements a bevy plugin that adds the resources and systems to facilitate gamepad_device input

use crate::{
    core::{Axis, Input},
    gamepad_device::{
        gamepad_event_system, GamepadAxis, GamepadButton, GamepadEvent, GamepadEventRaw,
        GamepadSettings,
    },
};
use bevy_app::prelude::*;
use bevy_ecs::IntoQuerySystem;

/// Adds gamepad_device input to an App
#[derive(Default)]
pub struct GamepadInputPlugin;

impl Plugin for GamepadInputPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_event::<GamepadEvent>()
            .add_event::<GamepadEventRaw>()
            .init_resource::<GamepadSettings>()
            .init_resource::<Input<GamepadButton>>()
            .init_resource::<Axis<GamepadAxis>>()
            .init_resource::<Axis<GamepadButton>>()
            .add_system_to_stage(bevy_app::stage::EVENT, gamepad_event_system.system());
    }
}

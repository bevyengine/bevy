//! Gamepad Input Plugin Module.
//! Implements a bevy plugin that adds the resources and systems to facilitate gamepad input

use crate::{
    axis::Axis,
    gamepad::{
        gamepad_event_system, GamepadAxis, GamepadButton, GamepadEvent, GamepadEventRaw,
        GamepadSettings,
    },
    input::Input,
};
use bevy_app::prelude::*;
use bevy_ecs::IntoQuerySystem;

/// Adds gamepad input to an App
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

//! Touch Input Plugin Module.
//! Implements a bevy plugin that adds the resources and systems to facilitate touch input

use crate::touch::{touch_screen_input_system, TouchInput, Touches};
use bevy_app::prelude::*;
use bevy_ecs::IntoQuerySystem;

/// Adds touch input to a bevy App
#[derive(Default)]
pub struct TouchInputPlugin;

impl Plugin for TouchInputPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_event::<TouchInput>()
            .init_resource::<Touches>()
            .add_system_to_stage(bevy_app::stage::EVENT, touch_screen_input_system.system());
    }
}

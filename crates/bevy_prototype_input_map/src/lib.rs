pub mod inputmap;
pub mod keyboard;

use bevy_app::prelude::*;
use bevy_input::{prelude::KeyCode, Input};
use bevy_ecs::IntoQuerySystem;

mod inputmap;
mod keyboard;
use crate::inputmap::InputMap;

#[derive(Default)]
pub struct InputMapPlugin;

impl Plugin for InputMapPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<Input<KeyCode>>()
            .init_resource::<InputMap>()
            .add_system_to_stage(
                bevy_app::stage::EVENT_UPDATE,
                InputMap::keyboard_input_map_system.system()
            )
            ;
    }
}

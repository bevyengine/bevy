mod converter;
mod gilrs_system;

use bevy_app::prelude::*;
use bevy_ecs::IntoQuerySystem;
use gilrs_system::{gilrs_startup_system, gilrs_update_system, GilrsArcMutexWrapper};

#[derive(Default)]
pub struct GilrsPlugin;

impl Plugin for GilrsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        match gilrs::Gilrs::new() {
            Ok(gilrs) => {
                app.add_resource(GilrsArcMutexWrapper::new(gilrs))
                    .add_startup_system(gilrs_startup_system.system())
                    .add_system_to_stage(stage::EVENT_UPDATE, gilrs_update_system.system());
            }
            Err(err) => log::error!("Failed to start Gilrs. {}", err),
        }
    }
}

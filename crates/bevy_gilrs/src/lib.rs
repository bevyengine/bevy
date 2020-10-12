mod converter;
mod gilrs_system;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use gilrs_system::{gilrs_startup_system, gilrs_update_system};

#[derive(Default)]
pub struct GilrsPlugin;

impl Plugin for GilrsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        match gilrs::Gilrs::new() {
            Ok(gilrs) => {
                app.add_thread_local_resource(gilrs)
                    .add_startup_system(gilrs_startup_system.thread_local_system())
                    .add_system_to_stage(
                        stage::EVENT_UPDATE,
                        gilrs_update_system.thread_local_system(),
                    );
            }
            Err(err) => log::error!("Failed to start Gilrs. {}", err),
        }
    }
}

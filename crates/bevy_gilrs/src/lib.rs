mod converter;
mod gilrs_system;

use bevy_app::{AppBuilder, CoreStage, Plugin, StartupStage};
use bevy_ecs::system::IntoExclusiveSystem;
use bevy_utils::tracing::error;
use gilrs::GilrsBuilder;
use gilrs_system::{gilrs_event_startup_system, gilrs_event_system};

#[derive(Default)]
pub struct GilrsPlugin;

impl Plugin for GilrsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        match GilrsBuilder::new()
            .with_default_filters(false)
            .set_update_state(false)
            .build()
        {
            Ok(gilrs) => {
                app.insert_non_send_resource(gilrs)
                    .add_startup_system_to_stage(
                        StartupStage::PreStartup,
                        gilrs_event_startup_system.exclusive_system(),
                    )
                    .add_system_to_stage(
                        CoreStage::PreUpdate,
                        gilrs_event_system.exclusive_system(),
                    );
            }
            Err(err) => error!("Failed to start Gilrs. {}", err),
        }
    }
}

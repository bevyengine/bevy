mod converter;
mod gilrs_system;

use bevy_app::{App, CoreStage, Plugin, StartupStage};
use bevy_ecs::prelude::{IntoExclusiveSystem, StageConfig, StartupConfig};
use bevy_utils::tracing::error;
use gilrs::GilrsBuilder;
use gilrs_system::{gilrs_event_startup_system, gilrs_event_system};

#[derive(Default)]
pub struct GilrsPlugin;

impl Plugin for GilrsPlugin {
    fn build(&self, app: &mut App) {
        match GilrsBuilder::new()
            .with_default_filters(false)
            .set_update_state(false)
            .build()
        {
            Ok(gilrs) => {
                app.insert_non_send_resource(gilrs)
                    .add_exclusive(
                        gilrs_event_startup_system
                            .exclusive_system()
                            .startup()
                            .stage(StartupStage::PreStartup),
                    )
                    .add_exclusive(gilrs_event_system.stage(CoreStage::PreUpdate));
            }
            Err(err) => error!("Failed to start Gilrs. {}", err),
        }
    }
}

use serde::Deserialize;

use crate::{app::AppExit, AppBuilder};
use bevy_ecs::system::IntoSystem;

/// Debug configuration, to help with Bevy development
#[derive(Deserialize)]
pub struct CiTestingConfig {
    /// Number of frames after wich Bevy should exit
    pub exit_after: Option<u32>,
}

fn debug_exit_after(
    mut current_frame: bevy_ecs::prelude::Local<u32>,
    debug_config: bevy_ecs::prelude::Res<CiTestingConfig>,
    mut app_exit_events: crate::EventWriter<AppExit>,
) {
    if let Some(exit_after) = debug_config.exit_after {
        if *current_frame > exit_after {
            app_exit_events.send(AppExit);
        }
    }
    *current_frame += 1;
}

pub(crate) fn setup_app(app_builder: &mut AppBuilder) -> &mut AppBuilder {
    let filename =
        std::env::var("DEBUG_CONFIG").unwrap_or_else(|_| "ci_testing_config.ron".to_string());
    let config: CiTestingConfig = ron::from_str(
        &std::fs::read_to_string(filename).expect("error reading CI testing configuration file"),
    )
    .expect("error deserializing CI testing configuration file");
    app_builder
        .insert_resource(config)
        .add_system(debug_exit_after.system());

    app_builder
}

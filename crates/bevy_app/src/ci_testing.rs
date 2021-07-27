use serde::Deserialize;

use crate::{app::AppExit, App};

/// Configuration for automated testing on CI
#[derive(Deserialize)]
pub struct CiTestingConfig {
    /// Number of frames after wich Bevy should exit
    pub exit_after: Option<u32>,
}

fn ci_testing_exit_after(
    mut current_frame: bevy_ecs::prelude::Local<u32>,
    ci_testing_config: bevy_ecs::prelude::Res<CiTestingConfig>,
    mut app_exit_events: crate::EventWriter<AppExit>,
) {
    if let Some(exit_after) = ci_testing_config.exit_after {
        if *current_frame > exit_after {
            app_exit_events.send(AppExit);
        }
    }
    *current_frame += 1;
}

pub(crate) fn setup_app(app_builder: &mut App) -> &mut App {
    let filename =
        std::env::var("CI_TESTING_CONFIG").unwrap_or_else(|_| "ci_testing_config.ron".to_string());
    let config: CiTestingConfig = ron::from_str(
        &std::fs::read_to_string(filename).expect("error reading CI testing configuration file"),
    )
    .expect("error deserializing CI testing configuration file");
    app_builder
        .insert_resource(config)
        .add_system(ci_testing_exit_after);

    app_builder
}

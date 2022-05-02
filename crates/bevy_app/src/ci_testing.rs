use crate::{app::AppExit, App};
use serde::Deserialize;

/// A configuration struct for automated CI testing.
///
/// It gets used when the `bevy_ci_testing` feature is enabled to automatically
/// exit a Bevy app when run through the CI. This is needed because otherwise
/// Bevy apps would be stuck in the game loop and wouldn't allow the CI to progress.
#[derive(Deserialize)]
pub struct CiTestingConfig {
    /// The number of frames after which Bevy should exit.
    pub exit_after: Option<u32>,
}

fn ci_testing_exit_after(
    mut current_frame: bevy_ecs::prelude::Local<u32>,
    ci_testing_config: bevy_ecs::prelude::Res<CiTestingConfig>,
    mut app_exit_events: bevy_ecs::event::EventWriter<AppExit>,
) {
    if let Some(exit_after) = ci_testing_config.exit_after {
        if *current_frame > exit_after {
            app_exit_events.send(AppExit);
        }
    }
    *current_frame += 1;
}

pub(crate) fn setup_app(app: &mut App) -> &mut App {
    let filename =
        std::env::var("CI_TESTING_CONFIG").unwrap_or_else(|_| "ci_testing_config.ron".to_string());
    let config: CiTestingConfig = ron::from_str(
        &std::fs::read_to_string(filename).expect("error reading CI testing configuration file"),
    )
    .expect("error deserializing CI testing configuration file");
    app.insert_resource(config)
        .add_system(ci_testing_exit_after);

    app
}

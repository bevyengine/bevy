//! Utilities for testing in CI environments.

use crate::{app::AppExit, App, Update};
use serde::Deserialize;

use bevy_ecs::prelude::Resource;
use bevy_utils::tracing::info;

/// A configuration struct for automated CI testing.
///
/// It gets used when the `bevy_ci_testing` feature is enabled to automatically
/// exit a Bevy app when run through the CI. This is needed because otherwise
/// Bevy apps would be stuck in the game loop and wouldn't allow the CI to progress.
#[derive(Deserialize, Resource)]
pub struct CiTestingConfig {
    /// The number of frames after which Bevy should exit.
    pub exit_after: Option<u32>,
    /// The time in seconds to update for each frame.
    pub frame_time: Option<f32>,
    /// Frames at which to capture a screenshot.
    #[serde(default)]
    pub screenshot_frames: Vec<u32>,
}

fn ci_testing_exit_after(
    mut current_frame: bevy_ecs::prelude::Local<u32>,
    ci_testing_config: bevy_ecs::prelude::Res<CiTestingConfig>,
    mut app_exit_events: bevy_ecs::event::EventWriter<AppExit>,
) {
    if let Some(exit_after) = ci_testing_config.exit_after {
        if *current_frame > exit_after {
            app_exit_events.send(AppExit);
            info!("Exiting after {} frames. Test successful!", exit_after);
        }
    }
    *current_frame += 1;
}

pub(crate) fn setup_app(app: &mut App) -> &mut App {
    #[cfg(not(target_arch = "wasm32"))]
    let config: CiTestingConfig = {
        let filename = std::env::var("CI_TESTING_CONFIG")
            .unwrap_or_else(|_| "ci_testing_config.ron".to_string());
        ron::from_str(
            &std::fs::read_to_string(filename)
                .expect("error reading CI testing configuration file"),
        )
        .expect("error deserializing CI testing configuration file")
    };
    #[cfg(target_arch = "wasm32")]
    let config: CiTestingConfig = {
        let config = include_str!("../../../ci_testing_config.ron");
        ron::from_str(config).expect("error deserializing CI testing configuration file")
    };

    app.insert_resource(config)
        .add_systems(Update, ci_testing_exit_after);

    app
}

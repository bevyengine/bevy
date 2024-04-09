//! Utilities for testing in CI environments.

use bevy_app::{App, AppExit, Update};
use bevy_ecs::{
    entity::Entity,
    prelude::{resource_exists, Resource},
    query::With,
    schedule::IntoSystemConfigs,
    system::{Local, Query, Res, ResMut},
};
use bevy_render::view::screenshot::ScreenshotManager;
use bevy_time::TimeUpdateStrategy;
use bevy_utils::{tracing::info, Duration};
use bevy_window::PrimaryWindow;
use serde::Deserialize;

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

    if let Some(frame_time) = config.frame_time {
        app.world_mut()
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                frame_time,
            )));
    }

    app.insert_resource(config).add_systems(
        Update,
        (
            ci_testing_exit_after,
            ci_testing_screenshot_at.run_if(resource_exists::<ScreenshotManager>),
        ),
    );

    app
}

fn ci_testing_screenshot_at(
    mut current_frame: Local<u32>,
    ci_testing_config: Res<CiTestingConfig>,
    mut screenshot_manager: ResMut<ScreenshotManager>,
    main_window: Query<Entity, With<PrimaryWindow>>,
) {
    if ci_testing_config
        .screenshot_frames
        .contains(&*current_frame)
    {
        info!("Taking a screenshot at frame {}.", *current_frame);
        let path = format!("./screenshot-{}.png", *current_frame);
        screenshot_manager
            .save_screenshot_to_disk(main_window.single(), path)
            .unwrap();
    }
    *current_frame += 1;
}

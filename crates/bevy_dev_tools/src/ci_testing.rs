//! Utilities for testing in CI environments.

use bevy_app::{App, AppExit, Update};
use bevy_ecs::{
    entity::Entity, event::Event, prelude::Resource, query::With, system::Local, world::World,
};
use bevy_render::view::screenshot::ScreenshotManager;
use bevy_time::TimeUpdateStrategy;
use bevy_utils::{
    tracing::{debug, info, warn},
    Duration,
};
use bevy_window::PrimaryWindow;
use serde::Deserialize;

/// A configuration struct for automated CI testing.
///
/// It gets used when the `bevy_ci_testing` feature is enabled to automatically
/// exit a Bevy app when run through the CI. This is needed because otherwise
/// Bevy apps would be stuck in the game loop and wouldn't allow the CI to progress.
#[derive(Deserialize, Resource)]
struct CiTestingConfig {
    /// The setup for this test.
    #[serde(default)]
    setup: CiTestingSetup,
    /// Events to send, with their associated frame.
    #[serde(default)]
    events: Vec<CiTestingEvent>,
}

/// Setup for a test.
#[derive(Deserialize, Default)]
struct CiTestingSetup {
    /// The time in seconds to update for each frame.
    pub frame_time: Option<f32>,
}

/// An event to send at a given frame, used for CI testing.
#[derive(Deserialize, Event)]
pub struct CiTestingEvent(u32, String);

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

    if let Some(frame_time) = config.setup.frame_time {
        app.world_mut()
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                frame_time,
            )));
    }

    app.insert_resource(config).add_systems(Update, send_events);

    app
}

fn send_events(world: &mut World, mut current_frame: Local<u32>) {
    let mut config = world.resource_mut::<CiTestingConfig>();

    let events = std::mem::take(&mut config.events);
    let (to_run, remaining): (Vec<_>, _) = events
        .into_iter()
        .partition(|event| event.0 == *current_frame);
    config.events = remaining;

    for event in to_run {
        debug!("Sending event: {}", event.1);
        match event.1.as_str() {
            "AppExit::Success" => {
                world.send_event(AppExit::Success);
                info!("Exiting after {} frames. Test successful!", event.0);
            }
            "Screenshot" => {
                let mut primary_window_query =
                    world.query_filtered::<Entity, With<PrimaryWindow>>();
                let Ok(main_window) = primary_window_query.get_single(world) else {
                    warn!("Requesting screenshot, but PrimaryWindow is not available");
                    continue;
                };
                let Some(mut screenshot_manager) = world.get_resource_mut::<ScreenshotManager>()
                else {
                    warn!("Requesting screenshot, but ScreenshotManager is not available");
                    continue;
                };
                let path = format!("./screenshot-{}.png", event.0);
                screenshot_manager
                    .save_screenshot_to_disk(main_window, path)
                    .unwrap();
                info!("Took a screenshot at frame {}.", event.0);
            }
            _ => {
                world.send_event(event);
            }
        }
    }

    *current_frame += 1;
}

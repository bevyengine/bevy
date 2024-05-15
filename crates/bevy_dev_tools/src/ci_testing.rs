//! Utilities for testing in CI environments.

use bevy_app::{App, AppExit, Update};
use bevy_ecs::prelude::*;
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
    events: Vec<CiTestingEventOnFrame>,
}

/// Setup for a test.
#[derive(Deserialize, Default)]
struct CiTestingSetup {
    /// The time in seconds to update for each frame.
    /// Set with the `TimeUpdateStrategy::ManualDuration(f32)` resource.
    pub fixed_frame_time: Option<f32>,
}

/// An event to send at a given frame, used for CI testing.
#[derive(Deserialize)]
pub struct CiTestingEventOnFrame(u32, CiTestingEvent);

/// An event to send, used for CI testing.
#[derive(Deserialize, Debug)]
enum CiTestingEvent {
    Screenshot,
    AppExit,
    Custom(String),
}

/// A custom event that can be configured from a configuration file for CI testing.
#[derive(Event)]
pub struct CiTestingCustomEvent(pub String);

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

    if let Some(fixed_frame_time) = config.setup.fixed_frame_time {
        app.world_mut()
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                fixed_frame_time,
            )));
    }

    app.add_event::<CiTestingCustomEvent>()
        .insert_resource(config)
        .add_systems(Update, send_events);

    app
}

fn send_events(world: &mut World, mut current_frame: Local<u32>) {
    let mut config = world.resource_mut::<CiTestingConfig>();

    let events = std::mem::take(&mut config.events);
    let (to_run, remaining): (Vec<_>, _) = events
        .into_iter()
        .partition(|event| event.0 == *current_frame);
    config.events = remaining;

    for CiTestingEventOnFrame(_, event) in to_run {
        debug!("Handling event: {:?}", event);
        match event {
            CiTestingEvent::AppExit => {
                world.send_event(AppExit::Success);
                info!("Exiting after {} frames. Test successful!", *current_frame);
            }
            CiTestingEvent::Screenshot => {
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
                let path = format!("./screenshot-{}.png", *current_frame);
                screenshot_manager
                    .save_screenshot_to_disk(main_window, path)
                    .unwrap();
                info!("Took a screenshot at frame {}.", *current_frame);
            }
            // Custom events are forwarded to the world.
            CiTestingEvent::Custom(event_string) => {
                world.send_event(CiTestingCustomEvent(event_string));
            }
        }
    }

    *current_frame += 1;
}

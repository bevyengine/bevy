use bevy_ecs::prelude::*;
use serde::Deserialize;

/// A configuration struct for automated CI testing.
///
/// It gets used when the `bevy_ci_testing` feature is enabled to automatically
/// exit a Bevy app when run through the CI. This is needed because otherwise
/// Bevy apps would be stuck in the game loop and wouldn't allow the CI to progress.
#[derive(Deserialize, Resource)]
pub struct CiTestingConfig {
    /// The setup for this test.
    #[serde(default)]
    pub setup: CiTestingSetup,
    /// Events to send, with their associated frame.
    #[serde(default)]
    pub events: Vec<CiTestingEventOnFrame>,
}

/// Setup for a test.
#[derive(Deserialize, Default)]
pub struct CiTestingSetup {
    /// The time in seconds to update for each frame.
    /// Set with the `TimeUpdateStrategy::ManualDuration(f32)` resource.
    pub fixed_frame_time: Option<f32>,
}

/// An event to send at a given frame, used for CI testing.
#[derive(Deserialize)]
pub struct CiTestingEventOnFrame(pub u32, pub CiTestingEvent);

/// An event to send, used for CI testing.
#[derive(Deserialize, Debug)]
pub enum CiTestingEvent {
    Screenshot,
    AppExit,
    Custom(String),
}

/// A custom event that can be configured from a configuration file for CI testing.
#[derive(Event)]
pub struct CiTestingCustomEvent(pub String);

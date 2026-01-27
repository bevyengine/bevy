use bevy_ecs::prelude::*;
use bevy_math::{Quat, Vec3};
use serde::Deserialize;

/// A configuration struct for automated CI testing.
///
/// It gets used when the `bevy_ci_testing` feature is enabled to automatically
/// exit a Bevy app when run through the CI. This is needed because otherwise
/// Bevy apps would be stuck in the game loop and wouldn't allow the CI to progress.
#[derive(Deserialize, Resource, PartialEq, Debug, Default, Clone)]
pub struct CiTestingConfig {
    /// The setup for this test.
    #[serde(default)]
    pub setup: CiTestingSetup,
    /// Events to send, with their associated frame.
    #[serde(default)]
    pub events: Vec<CiTestingEventOnFrame>,
}

/// Setup for a test.
#[derive(Deserialize, Default, PartialEq, Debug, Clone)]
pub struct CiTestingSetup {
    /// The amount of time in seconds between frame updates.
    ///
    /// This is set through the [`TimeUpdateStrategy::ManualDuration`] resource.
    ///
    /// [`TimeUpdateStrategy::ManualDuration`]: bevy_time::TimeUpdateStrategy::ManualDuration
    pub fixed_frame_time: Option<f32>,
}

/// An event to send at a given frame, used for CI testing.
#[derive(Deserialize, PartialEq, Debug, Clone)]
pub struct CiTestingEventOnFrame(pub u32, pub CiTestingEvent);

/// An event to send, used for CI testing.
#[derive(Deserialize, PartialEq, Debug, Clone)]
pub enum CiTestingEvent {
    /// Takes a screenshot of the entire screen, and saves the results to
    /// `screenshot-{current_frame}.png`.
    Screenshot,
    /// Takes a screenshot of the entire screen, saves the results to
    /// `screenshot-{current_frame}.png`, and exits once the screenshot is taken.
    ScreenshotAndExit,
    /// Takes a screenshot of the entire screen, and saves the results to
    /// `screenshot-{name}.png`.
    NamedScreenshot(String),
    /// Stops the program by sending [`AppExit::Success`].
    ///
    /// [`AppExit::Success`]: bevy_app::AppExit::Success
    AppExit,
    /// Starts recording the screen.
    StartScreenRecording,
    /// Stops recording the screen.
    StopScreenRecording,
    /// Smoothly moves the camera to the given position.
    MoveCamera {
        /// Position to move the camera to.
        translation: Vec3,
        /// Rotation to move the camera to.
        rotation: Quat,
    },
    /// Sends a [`CiTestingCustomEvent`] using the given [`String`].
    Custom(String),
}

/// A custom event that can be configured from a configuration file for CI testing.
#[derive(Message)]
pub struct CiTestingCustomEvent(pub String);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize() {
        const INPUT: &str = r#"
(
    setup: (
        fixed_frame_time: Some(0.03),
    ),
    events: [
        (100, Custom("Hello, world!")),
        (200, Screenshot),
        (300, AppExit),
    ],
)"#;

        let expected = CiTestingConfig {
            setup: CiTestingSetup {
                fixed_frame_time: Some(0.03),
            },
            events: vec![
                CiTestingEventOnFrame(100, CiTestingEvent::Custom("Hello, world!".into())),
                CiTestingEventOnFrame(200, CiTestingEvent::Screenshot),
                CiTestingEventOnFrame(300, CiTestingEvent::AppExit),
            ],
        };

        let config: CiTestingConfig = ron::from_str(INPUT).unwrap();

        assert_eq!(config, expected);
    }
}

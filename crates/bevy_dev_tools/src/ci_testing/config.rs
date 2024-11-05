use bevy_ecs::prelude::*;
use serde::Deserialize;

/// A configuration struct for automated CI testing.
///
/// It gets used when the `bevy_ci_testing` feature is enabled to automatically
/// exit a Bevy app when run through the CI. This is needed because otherwise
/// Bevy apps would be stuck in the game loop and wouldn't allow the CI to progress.
#[derive(Deserialize, Resource, PartialEq, Debug)]
pub struct CiTestingConfig {
    /// The setup for this test.
    #[serde(default)]
    pub setup: CiTestingSetup,
    /// Events to send, with their associated frame.
    #[serde(default)]
    pub events: Vec<CiTestingEventOnFrame>,
}

/// Setup for a test.
#[derive(Deserialize, Default, PartialEq, Debug)]
pub struct CiTestingSetup {
    /// The amount of time in seconds between frame updates.
    ///
    /// This is set through the [`TimeUpdateStrategy::ManualDuration`] resource.
    ///
    /// [`TimeUpdateStrategy::ManualDuration`]: bevy_time::TimeUpdateStrategy::ManualDuration
    pub fixed_frame_time: Option<f32>,
}

/// An event to send at a given frame, used for CI testing.
#[derive(Deserialize, PartialEq, Debug)]
pub struct CiTestingEventOnFrame(pub u32, pub CiTestingEvent);

/// An event to send, used for CI testing.
#[derive(Deserialize, PartialEq, Debug)]
pub enum CiTestingEvent {
    /// Takes a screenshot of the entire screen, and saves the results to
    /// `screenshot-{current_frame}.png`.
    Screenshot,
    /// Stops the program by sending [`AppExit::Success`].
    ///
    /// [`AppExit::Success`]: bevy_app::AppExit::Success
    AppExit,
    /// Sends a [`CiTestingCustomEvent`] using the given [`String`].
    Custom(String),
}

/// A custom event that can be configured from a configuration file for CI testing.
#[derive(Event)]
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

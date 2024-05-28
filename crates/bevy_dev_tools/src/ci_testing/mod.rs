//! Utilities for testing in CI environments.

mod config;
mod systems;

pub use self::config::*;

use bevy_app::prelude::*;
use bevy_time::TimeUpdateStrategy;
use std::time::Duration;

/// A plugin that instruments continuous integration testing by automatically executing user-defined actions.
///
/// This plugin reads a [`ron`] file specified with the `CI_TESTING_CONFIG` environmental variable
/// (`ci_testing_config.ron` by default) and executes its specified actions. For a reference of the
/// allowed configuration, see [`CiTestingConfig`].
///
/// This plugin is included within `DefaultPlugins` and `MinimalPlugins` when the `bevy_ci_testing`
/// feature is enabled. It is recommended to only used this plugin during testing (manual or
/// automatic), and disable it during regular development and for production builds.
pub struct CiTestingPlugin;

impl Plugin for CiTestingPlugin {
    fn build(&self, app: &mut App) {
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
            let config = include_str!("../../../../ci_testing_config.ron");
            ron::from_str(config).expect("error deserializing CI testing configuration file")
        };

        // Configure a fixed frame time if specified.
        if let Some(fixed_frame_time) = config.setup.fixed_frame_time {
            app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                fixed_frame_time,
            )));
        }

        app.add_event::<CiTestingCustomEvent>()
            .insert_resource(config)
            .add_systems(Update, systems::send_events);
    }
}

#![warn(missing_docs)]
//! This crate provides additional utilities for the [Bevy game engine](https://bevyengine.org),
//! focused on improving developer experience.

use bevy_app::prelude::*;
#[cfg(feature = "bevy_ci_testing")]
pub mod ci_testing;

/// Enables developer tools in an [`App`].
/// This plugin is part of the [`DefaultPlugins`](https://docs.rs/bevy/latest/bevy/struct.DefaultPlugins.html), and enabled by default.
///
/// It's recommended to disable it in a release. To disable it, you can either:
///
/// - Remove it when adding [`DefaultPlugins`](https://docs.rs/bevy/latest/bevy/struct.DefaultPlugins.html):
/// ```no_run
/// # use bevy_app::{App, NoopPluginGroup as DefaultPlugins, PluginGroup};
/// # use bevy_dev_tools::{DevToolsPlugin};
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins.build().disable::<DevToolsPlugin>())
///         .run();
/// }
/// ```
///
/// - Disable the feature:
/// Disable default features from Bevy, and don't enable the feature `bevy_dev_tools`.
pub struct DevToolsPlugin;

impl Plugin for DevToolsPlugin {
    fn build(&self, _app: &mut App) {
        #[cfg(feature = "bevy_ci_testing")]
        {
            ci_testing::setup_app(_app);
        }
    }
}

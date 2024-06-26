#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

//! This crate provides additional utilities for the [Bevy game engine](https://bevyengine.org),
//! focused on improving developer experience.

use bevy_app::prelude::*;

#[cfg(feature = "bevy_ci_testing")]
pub mod ci_testing;

pub mod fps_overlay;

#[cfg(feature = "bevy_ui_debug")]
pub mod ui_debug_overlay;

pub mod states;

/// Enables developer tools in an [`App`]. This plugin is added automatically with `bevy_dev_tools`
/// feature.
///
/// Warning: It is not recommended to enable this in final shipped games or applications.
/// Dev tools provide a high level of access to the internals of your application,
/// and may interfere with ordinary use and gameplay.
///
/// To enable developer tools, you can either:
///
/// - Create a custom crate feature (e.g "`dev_mode`"), which enables the `bevy_dev_tools` feature
/// along with any other development tools you might be using:
///
/// ```toml
/// [feature]
/// dev_mode = ["bevy/bevy_dev_tools", "other_dev_tools"]
/// ```
///
/// - Use `--feature bevy/bevy_dev_tools` flag when using the `cargo run` command:
///
/// `cargo run --features bevy/bevy_dev_tools`
///
/// - Add the `bevy_dev_tools` feature to the bevy dependency in your `Cargo.toml` file:
///
/// `features = ["bevy_dev_tools"]`
///
///  Note: The third method is not recommended, as it requires you to remove the feature before
///  creating a build for release to the public.
pub struct DevToolsPlugin;

impl Plugin for DevToolsPlugin {
    fn build(&self, _app: &mut App) {}
}

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

/// Contains the `DevTool` trait and default dev commands for dev tools
pub mod dev_tool;
/// Contains the `DevTool` trait and reflect registration
pub mod dev_command;
/// Contains the `Toggable` trait and commands for enabling/disabling Toggable dev tools
pub mod toggable;
/// Contains plugins for enable cli parsing and executing dev commands
pub mod cli_toolbox;

/// Macros for the `bevy_dev_tools` plugin
pub use bevy_dev_tools_macros::*;

/// Macros for the `bevy_dev_tools` plugin
pub mod prelude {
    pub use bevy_dev_tools_macros::*;
    pub use crate::dev_tool::*;
    pub use crate::dev_command::*;
    pub use crate::toggable::*;
    pub use crate::cli_toolbox::*;
}

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
    fn build(&self, _app: &mut App) {
        #[cfg(feature = "bevy_ci_testing")]
        {
            ci_testing::setup_app(_app);
        }
    }
}

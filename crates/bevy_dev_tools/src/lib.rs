#![warn(missing_docs)]
//! This crate provides additional utilities for the [Bevy game engine](https://bevyengine.org),
//! aimed at improving developer experience.

use bevy_app::prelude::*;
#[cfg(feature = "bevy_ci_testing")]
pub mod ci_testing;

/// Adds developer tools to an App.
pub struct DevToolsPlugin;

impl Plugin for DevToolsPlugin {
    fn build(&self, _app: &mut App) {
        #[cfg(feature = "bevy_ci_testing")]
        {
            ci_testing::setup_app(_app);
        }
    }
}

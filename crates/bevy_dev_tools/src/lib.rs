#![warn(missing_docs)]
//! This crate provides additional utilities for the [Bevy game engine](https://bevyengine.org),
//! aimed at improving developer experience.

use bevy_app::prelude::*;

/// Adds developer tools to an App.
pub struct DevToolsPlugin;

impl Plugin for DevToolsPlugin {
    fn build(&self, _app: &mut App) {}
}

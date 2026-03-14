use bevy_app::{App, Plugin};
use bevy_shader::load_shader_library;

/// A plugin for WGSL color space utility functions
pub struct ColorSpacePlugin;

impl Plugin for ColorSpacePlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "color_space.wgsl");
    }
}

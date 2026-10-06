mod point_light;

use bevy_app::{App, Plugin};
use bevy_extract::extract_component::ExtractComponentPlugin;
use bevy_render::RenderApp;
use bevy_sprite_light::PointLight2d;

/// Adds 2d lighting support.
#[derive(Default)]
pub struct Lighting2dPlugin;

impl Plugin for Lighting2dPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<PointLight2d, RenderApp, Self>::default());
    }
}

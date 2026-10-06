mod ambient_light;
mod point_light;

use bevy_app::{App, Plugin};
use bevy_extract::extract_component::ExtractComponentPlugin;
use bevy_render::{extract_resource::ExtractResourcePlugin, RenderApp};
use bevy_sprite_light::{GlobalAmbientLight2d, PointLight2d};

/// Adds 2d lighting support.
#[derive(Default)]
pub struct Lighting2dPlugin;

impl Plugin for Lighting2dPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GlobalAmbientLight2d>().add_plugins((
            ExtractComponentPlugin::<PointLight2d, RenderApp, Self>::default(),
            ExtractResourcePlugin::<GlobalAmbientLight2d, Self>::default(),
        ));
    }
}

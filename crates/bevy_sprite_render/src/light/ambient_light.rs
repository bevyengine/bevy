use bevy_render::{extract_resource::ExtractResource, RenderApp};
use bevy_sprite_light::GlobalAmbientLight2d;

use super::Lighting2dPlugin;

impl ExtractResource<RenderApp, Lighting2dPlugin> for GlobalAmbientLight2d {
    type Source = GlobalAmbientLight2d;

    fn extract_resource(source: &Self::Source) -> Self {
        source.clone()
    }
}

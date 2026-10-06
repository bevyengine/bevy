use bevy_camera::visibility::InheritedVisibility;
use bevy_color::Color;
use bevy_ecs::{component::Component, query::QueryItem};
use bevy_render::{extract_component::ExtractComponent, sync_component::SyncComponent, RenderApp};
use bevy_sprite_light::PointLight2d;
use bevy_transform::components::GlobalTransform;

use super::Lighting2dPlugin;

#[derive(Component)]
pub struct ExtractedPointLight2d {
    pub color: Color,
    pub transform: GlobalTransform,
    pub range: f32,
    pub intensity: f32,
}

impl SyncComponent<RenderApp, Lighting2dPlugin> for PointLight2d {
    type Target = ExtractedPointLight2d;
}

impl ExtractComponent<RenderApp, Lighting2dPlugin> for PointLight2d {
    type QueryData = (
        &'static PointLight2d,
        &'static GlobalTransform,
        &'static InheritedVisibility,
    );
    type QueryFilter = ();
    type Out = ExtractedPointLight2d;

    fn extract_component(
        (light, transform, visibility): QueryItem<'_, '_, Self::QueryData>,
    ) -> Option<Self::Out> {
        if !visibility.get() {
            return None;
        }
        Some(ExtractedPointLight2d {
            color: light.color,
            transform: *transform,
            range: light.range,
            intensity: light.intensity,
        })
    }
}

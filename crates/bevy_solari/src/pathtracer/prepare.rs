use super::Pathtracer;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    system::{Commands, Query, Res, ResMut},
};
use bevy_render::{
    camera::ExtractedCamera,
    render_resource::{
        Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    },
    renderer::RenderDevice,
    texture::{CachedTexture, TextureCache},
};

#[derive(Component)]
pub struct PathtracerAccumulationTexture(pub CachedTexture);

pub fn prepare_pathtracer_accumulation_texture(
    query: Query<(Entity, &ExtractedCamera), With<Pathtracer>>,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    mut commands: Commands,
) {
    for (entity, camera) in &query {
        let Some(viewport) = camera.physical_viewport_size else {
            continue;
        };

        let descriptor = TextureDescriptor {
            label: Some("pathtracer_accumulation_texture"),
            size: Extent3d {
                width: viewport.x,
                height: viewport.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba32Float,
            usage: TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        };

        commands
            .entity(entity)
            .insert(PathtracerAccumulationTexture(
                texture_cache.get(&render_device, descriptor),
            ));
    }
}

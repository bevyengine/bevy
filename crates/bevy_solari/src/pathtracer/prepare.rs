use super::Pathtracer;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    system::{Commands, Query, Res, ResMut},
};
use bevy_image::ToExtents;
use bevy_render::{
    camera::ViewTargetInfo,
    render_resource::{TextureDescriptor, TextureDimension, TextureFormat, TextureUsages},
    renderer::RenderDevice,
    texture::{CachedTexture, TextureCache},
};

#[derive(Component)]
pub struct PathtracerAccumulationTexture(pub CachedTexture);

pub fn prepare_pathtracer_accumulation_texture(
    query: Query<(Entity, &ViewTargetInfo), With<Pathtracer>>,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    mut commands: Commands,
) {
    for (entity, target_info) in &query {
        let descriptor = TextureDescriptor {
            label: Some("pathtracer_accumulation_texture"),
            size: target_info.size.to_extents(),
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

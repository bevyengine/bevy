use bevy_core_pipeline::core_3d::{AlphaMask3d, Opaque3d, Transmissive3d, Transparent3d};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    system::{Commands, Query, Res, ResMut},
};
use bevy_image::{BevyDefault, ToExtents};
use bevy_platform::collections::HashMap;
use bevy_render::{
    camera::ExtractedCamera,
    render_phase::{ViewBinnedRenderPhases, ViewSortedRenderPhases},
    render_resource::{
        FilterMode, Sampler, SamplerDescriptor, Texture, TextureDescriptor, TextureDimension,
        TextureFormat, TextureUsages, TextureView,
    },
    renderer::RenderDevice,
    texture::TextureCache,
    view::{ExtractedView, ViewTarget},
};

use crate::ScreenSpaceTransmission;

#[derive(Component)]
pub struct ViewTransmissionTexture {
    pub texture: Texture,
    pub view: TextureView,
    pub sampler: Sampler,
}

pub fn prepare_core_3d_transmission_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    opaque_3d_phases: Res<ViewBinnedRenderPhases<Opaque3d>>,
    alpha_mask_3d_phases: Res<ViewBinnedRenderPhases<AlphaMask3d>>,
    transmissive_3d_phases: Res<ViewSortedRenderPhases<Transmissive3d>>,
    transparent_3d_phases: Res<ViewSortedRenderPhases<Transparent3d>>,
    views_3d: Query<(
        Entity,
        &ExtractedCamera,
        &ScreenSpaceTransmission,
        &ExtractedView,
    )>,
) {
    let mut textures = <HashMap<_, _>>::default();
    for (entity, camera, transmission, view) in &views_3d {
        if !opaque_3d_phases.contains_key(&view.retained_view_entity)
            || !alpha_mask_3d_phases.contains_key(&view.retained_view_entity)
            || !transparent_3d_phases.contains_key(&view.retained_view_entity)
        {
            continue;
        };

        let Some(transmissive_3d_phase) = transmissive_3d_phases.get(&view.retained_view_entity)
        else {
            continue;
        };

        let Some(physical_target_size) = camera.physical_target_size else {
            continue;
        };

        // Don't prepare a transmission texture if the number of steps is set to 0
        if transmission.screen_space_specular_transmission_steps == 0 {
            continue;
        }

        // Don't prepare a transmission texture if there are no transmissive items to render
        if transmissive_3d_phase.items.is_empty() {
            continue;
        }

        let cached_texture = textures
            .entry(camera.target.clone())
            .or_insert_with(|| {
                let usage = TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST;

                let format = if view.hdr {
                    ViewTarget::TEXTURE_FORMAT_HDR
                } else {
                    TextureFormat::bevy_default()
                };

                let descriptor = TextureDescriptor {
                    label: Some("view_transmission_texture"),
                    // The size of the transmission texture
                    size: physical_target_size.to_extents(),
                    mip_level_count: 1,
                    sample_count: 1, // No need for MSAA, as we'll only copy the main texture here
                    dimension: TextureDimension::D2,
                    format,
                    usage,
                    view_formats: &[],
                };

                texture_cache.get(&render_device, descriptor)
            })
            .clone();

        let sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("view_transmission_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });

        commands.entity(entity).insert(ViewTransmissionTexture {
            texture: cached_texture.texture,
            view: cached_texture.default_view,
            sampler,
        });
    }
}

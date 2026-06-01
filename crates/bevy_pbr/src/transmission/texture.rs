use bevy_core_pipeline::core_3d::{AlphaMask3d, Opaque3d, Transparent3d};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    system::{Commands, Query, Res, ResMut},
};
use bevy_image::ToExtents;
use bevy_platform::collections::HashMap;
use bevy_render::{
    camera::ExtractedCamera,
    render_phase::{ViewBinnedRenderPhases, ViewSortedRenderPhases},
    render_resource::{
        FilterMode, Sampler, SamplerDescriptor, Texture, TextureDescriptor, TextureDimension,
        TextureUsages, TextureView,
    },
    renderer::RenderDevice,
    texture::TextureCache,
    view::{ExtractedMultiview, ExtractedView},
};

use crate::{ScreenSpaceTransmission, Transmissive3d};

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
        Option<&ExtractedMultiview>,
    )>,
) {
    let mut textures = <HashMap<_, _>>::default();
    for (entity, camera, transmission, view, multiview) in &views_3d {
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
        if transmission.steps == 0 {
            continue;
        }

        // Don't prepare a transmission texture if there are no transmissive items to render
        if transmissive_3d_phase.items.is_empty() {
            continue;
        }

        // Allocate one layer per subview under multiview so the per-eye
        // transmissive pass can sample its own eye's pre-step main-texture
        // copy via the `D2Array` view at `mesh_view_bindings.rs:897-911`.
        // Non-multiview cameras stay 1-layer (byte-identical no-op).
        let view_count: u32 = multiview.map(|m| m.subviews.len() as u32).unwrap_or(1);
        let mut texture_size = physical_target_size.to_extents();
        texture_size.depth_or_array_layers = view_count;

        let cached_texture = textures
            .entry((camera.target.clone(), view_count))
            .or_insert_with(|| {
                let usage = TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST;

                let descriptor = TextureDescriptor {
                    label: Some("view_transmission_texture"),
                    // The size of the transmission texture
                    size: texture_size,
                    mip_level_count: 1,
                    sample_count: 1, // No need for MSAA, as we'll only copy the main texture here
                    dimension: TextureDimension::D2,
                    format: view.target_format,
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

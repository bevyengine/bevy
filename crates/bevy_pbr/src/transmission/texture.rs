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
    erased_render_asset::ErasedRenderAssets,
    render_phase::{PhaseItem, ViewBinnedRenderPhases, ViewSortedRenderPhases},
    render_resource::{
        FilterMode, Sampler, SamplerDescriptor, Texture, TextureDescriptor, TextureDimension,
        TextureUsages, TextureView,
    },
    renderer::RenderDevice,
    texture::TextureCache,
    view::ExtractedView,
};

use crate::{PreparedMaterial, RenderMaterialInstances, ScreenSpaceTransmission, Transmissive3d};

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
    render_materials: Res<ErasedRenderAssets<PreparedMaterial>>,
    material_instances: Res<RenderMaterialInstances>,
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
            commands.entity(entity).remove::<ViewTransmissionTexture>();
            continue;
        };

        let Some(transparent_3d_phase) = transparent_3d_phases.get(&view.retained_view_entity)
        else {
            commands.entity(entity).remove::<ViewTransmissionTexture>();
            continue;
        };

        let Some(transmissive_3d_phase) = transmissive_3d_phases.get(&view.retained_view_entity)
        else {
            commands.entity(entity).remove::<ViewTransmissionTexture>();
            continue;
        };

        let Some(physical_target_size) = camera.physical_target_size else {
            commands.entity(entity).remove::<ViewTransmissionTexture>();
            continue;
        };

        // Don't prepare a transmission texture if the number of steps is set to 0
        if transmission.steps == 0 {
            commands.entity(entity).remove::<ViewTransmissionTexture>();
            continue;
        }

        let transparent_phase_reads_view_transmission_texture =
            transparent_3d_phase.items.values().any(|transparent_item| {
                material_instances
                    .instances
                    .get(&transparent_item.main_entity())
                    .and_then(|material_instance| render_materials.get(material_instance.asset_id))
                    .is_some_and(|material| material.properties.reads_view_transmission_texture)
            });

        // Don't prepare a transmission texture if no phase will read from it.
        if transmissive_3d_phase.items.is_empty()
            && !transparent_phase_reads_view_transmission_texture
        {
            commands.entity(entity).remove::<ViewTransmissionTexture>();
            continue;
        }

        let cached_texture = textures
            .entry(camera.target.clone())
            .or_insert_with(|| {
                let usage = TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST;

                let descriptor = TextureDescriptor {
                    label: Some("view_transmission_texture"),
                    // The size of the transmission texture
                    size: physical_target_size.to_extents(),
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

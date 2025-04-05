use bevy_core_pipeline::{
    core_2d::Camera2d,
    tonemapping::{get_lut_bindings, DebandDither, Tonemapping, TonemappingLuts},
};
use bevy_ecs::{
    entity::Entity,
    query::{Has, With},
    system::{Commands, Query, Res, ResMut, SystemChangeTick},
};
use bevy_render::{
    batching::{no_gpu_preprocessing::BatchedInstanceBuffer, NoAutomaticBatching},
    globals::GlobalsBuffer,
    mesh::{Mesh2d, MeshTag},
    render_asset::RenderAssets,
    render_resource::BindGroupEntries,
    renderer::RenderDevice,
    sync_world::MainEntity,
    texture::{FallbackImage, GpuImage},
    view::{ExtractedView, Msaa, ViewUniforms, ViewVisibility},
    Extract,
};
use bevy_transform::components::GlobalTransform;

use super::{
    key::{tonemapping_pipeline_key, Mesh2dPipelineKey},
    pipeline::Mesh2dPipeline,
    render::{
        Material2dBindGroupId, Mesh2dBindGroup, Mesh2dTransforms, Mesh2dUniform,
        Mesh2dViewBindGroup, MeshFlags, RenderMesh2dInstance, RenderMesh2dInstances, ViewKeyCache,
        ViewSpecializationTicks,
    },
};

pub fn prepare_mesh2d_bind_group(
    mut commands: Commands,
    mesh2d_pipeline: Res<Mesh2dPipeline>,
    render_device: Res<RenderDevice>,
    mesh2d_uniforms: Res<BatchedInstanceBuffer<Mesh2dUniform>>,
) {
    if let Some(binding) = mesh2d_uniforms.instance_data_binding() {
        commands.insert_resource(Mesh2dBindGroup {
            value: render_device.create_bind_group(
                "mesh2d_bind_group",
                &mesh2d_pipeline.mesh_layout,
                &BindGroupEntries::single(binding),
            ),
        });
    }
}

pub fn prepare_mesh2d_view_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    mesh2d_pipeline: Res<Mesh2dPipeline>,
    view_uniforms: Res<ViewUniforms>,
    views: Query<(Entity, &Tonemapping), (With<ExtractedView>, With<Camera2d>)>,
    globals_buffer: Res<GlobalsBuffer>,
    tonemapping_luts: Res<TonemappingLuts>,
    images: Res<RenderAssets<GpuImage>>,
    fallback_image: Res<FallbackImage>,
) {
    let (Some(view_binding), Some(globals)) = (
        view_uniforms.uniforms.binding(),
        globals_buffer.buffer.binding(),
    ) else {
        return;
    };

    for (entity, tonemapping) in &views {
        let lut_bindings =
            get_lut_bindings(&images, &tonemapping_luts, tonemapping, &fallback_image);
        let view_bind_group = render_device.create_bind_group(
            "mesh2d_view_bind_group",
            &mesh2d_pipeline.view_layout,
            &BindGroupEntries::with_indices((
                (0, view_binding.clone()),
                (1, globals.clone()),
                (2, lut_bindings.0),
                (3, lut_bindings.1),
            )),
        );

        commands.entity(entity).insert(Mesh2dViewBindGroup {
            value: view_bind_group,
        });
    }
}

pub fn extract_mesh2d(
    mut render_mesh_instances: ResMut<RenderMesh2dInstances>,
    query: Extract<
        Query<(
            Entity,
            &ViewVisibility,
            &GlobalTransform,
            &Mesh2d,
            Option<&MeshTag>,
            Has<NoAutomaticBatching>,
        )>,
    >,
) {
    render_mesh_instances.clear();

    for (entity, view_visibility, transform, handle, tag, no_automatic_batching) in &query {
        if !view_visibility.get() {
            continue;
        }
        render_mesh_instances.insert(
            entity.into(),
            RenderMesh2dInstance {
                transforms: Mesh2dTransforms {
                    world_from_local: (&transform.affine()).into(),
                    flags: MeshFlags::empty().bits(),
                },
                mesh_asset_id: handle.0.id(),
                material_bind_group_id: Material2dBindGroupId::default(),
                automatic_batching: !no_automatic_batching,
                tag: tag.map_or(0, |i| **i),
            },
        );
    }
}

pub(super) fn check_views_need_specialization(
    mut view_key_cache: ResMut<ViewKeyCache>,
    mut view_specialization_ticks: ResMut<ViewSpecializationTicks>,
    views: Query<(
        &MainEntity,
        &ExtractedView,
        &Msaa,
        Option<&Tonemapping>,
        Option<&DebandDither>,
    )>,
    ticks: SystemChangeTick,
) {
    for (view_entity, view, msaa, tonemapping, dither) in &views {
        let mut view_key = Mesh2dPipelineKey::from_msaa_samples(msaa.samples())
            | Mesh2dPipelineKey::from_hdr(view.hdr);

        if !view.hdr {
            if let Some(tonemapping) = tonemapping {
                view_key |= Mesh2dPipelineKey::TONEMAP_IN_SHADER;
                view_key |= tonemapping_pipeline_key(*tonemapping);
            }
            if let Some(DebandDither::Enabled) = dither {
                view_key |= Mesh2dPipelineKey::DEBAND_DITHER;
            }
        }

        if !view_key_cache
            .get_mut(view_entity)
            .is_some_and(|current_key| *current_key == view_key)
        {
            view_key_cache.insert(*view_entity, view_key);
            view_specialization_ticks.insert(*view_entity, ticks.this_run());
        }
    }
}

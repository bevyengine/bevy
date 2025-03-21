mod bind_group;
mod commands;
mod components;
mod instancing;
mod pipeline;
mod resources;
mod shader_types;

use bevy_app::Plugin;
use bevy_asset::{load_internal_asset, weak_handle, Handle};
use bevy_core_pipeline::{
    core_2d::{AlphaMask2d, Opaque2d, Transparent2d},
    tonemapping::{DebandDither, Tonemapping},
};
use bevy_ecs::{
    entity::Entity,
    query::Has,
    schedule::IntoScheduleConfigs,
    system::{Query, ResMut, SystemChangeTick},
};
use bevy_render::{
    batching::{
        no_gpu_preprocessing::{
            self, batch_and_prepare_binned_render_phase, batch_and_prepare_sorted_render_phase,
            write_batched_instance_buffer, BatchedInstanceBuffer,
        },
        NoAutomaticBatching,
    },
    mesh::{Mesh2d, MeshTag},
    render_phase::sweep_old_entities,
    render_resource::{GpuArrayBuffer, Shader, ShaderDefVal, SpecializedMeshPipelines},
    renderer::RenderDevice,
    sync_world::MainEntity,
    view::{ExtractedView, Msaa, ViewVisibility},
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_transform::components::GlobalTransform;

use crate::material::rendering::Material2dBindGroupId;

use {
    bind_group::{prepare_mesh2d_bind_group, prepare_mesh2d_view_bind_groups},
    shader_types::Mesh2dUniform,
};
pub use {
    commands::{DrawMesh2d, SetMesh2dBindGroup, SetMesh2dViewBindGroup},
    components::Mesh2dTransforms,
    instancing::{RenderMesh2dInstance, RenderMesh2dInstances},
    pipeline::{Mesh2dPipeline, Mesh2dPipelineKey},
    resources::{ViewKeyCache, ViewSpecializationTicks},
};

const MESH2D_VERTEX_OUTPUT: Handle<Shader> = weak_handle!("71e279c7-85a0-46ac-9a76-1586cbf506d0");
const MESH2D_VIEW_TYPES_HANDLE: Handle<Shader> =
    weak_handle!("01087b0d-91e9-46ac-8628-dfe19a7d4b83");
const MESH2D_VIEW_BINDINGS_HANDLE: Handle<Shader> =
    weak_handle!("fbdd8b80-503d-4688-bcec-db29ab4620b2");
const MESH2D_TYPES_HANDLE: Handle<Shader> = weak_handle!("199f2089-6e99-4348-9bb1-d82816640a7f");
const MESH2D_BINDINGS_HANDLE: Handle<Shader> = weak_handle!("a7bd44cc-0580-4427-9a00-721cf386b6e4");
const MESH2D_FUNCTIONS_HANDLE: Handle<Shader> =
    weak_handle!("0d08ff71-68c1-4017-83e2-bfc34d285c51");
const MESH2D_SHADER_HANDLE: Handle<Shader> = weak_handle!("91a7602b-df95-4ea3-9d97-076abcb69d91");

#[derive(Default)]
pub struct Mesh2dRenderPlugin;

impl Plugin for Mesh2dRenderPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        load_internal_asset!(
            app,
            MESH2D_VERTEX_OUTPUT,
            "shaders/mesh2d_vertex_output.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            MESH2D_VIEW_TYPES_HANDLE,
            "shaders/mesh2d_view_types.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            MESH2D_VIEW_BINDINGS_HANDLE,
            "shaders/mesh2d_view_bindings.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            MESH2D_TYPES_HANDLE,
            "shaders/mesh2d_types.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            MESH2D_FUNCTIONS_HANDLE,
            "shaders/mesh2d_functions.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            MESH2D_SHADER_HANDLE,
            "shaders/mesh2d.wgsl",
            Shader::from_wgsl
        );

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ViewKeyCache>()
                .init_resource::<RenderMesh2dInstances>()
                .init_resource::<SpecializedMeshPipelines<Mesh2dPipeline>>()
                .add_systems(ExtractSchedule, extract_mesh2d)
                .add_systems(
                    Render,
                    (
                        (
                            sweep_old_entities::<Opaque2d>,
                            sweep_old_entities::<AlphaMask2d>,
                        )
                            .in_set(RenderSet::QueueSweep),
                        batch_and_prepare_binned_render_phase::<Opaque2d, Mesh2dPipeline>
                            .in_set(RenderSet::PrepareResources),
                        batch_and_prepare_binned_render_phase::<AlphaMask2d, Mesh2dPipeline>
                            .in_set(RenderSet::PrepareResources),
                        batch_and_prepare_sorted_render_phase::<Transparent2d, Mesh2dPipeline>
                            .in_set(RenderSet::PrepareResources),
                        write_batched_instance_buffer::<Mesh2dPipeline>
                            .in_set(RenderSet::PrepareResourcesFlush),
                        prepare_mesh2d_bind_group.in_set(RenderSet::PrepareBindGroups),
                        prepare_mesh2d_view_bind_groups.in_set(RenderSet::PrepareBindGroups),
                        no_gpu_preprocessing::clear_batched_cpu_instance_buffers::<Mesh2dPipeline>
                            .in_set(RenderSet::Cleanup)
                            .after(RenderSet::Render),
                    ),
                );
        }
    }

    fn finish(&self, app: &mut bevy_app::App) {
        let mut mesh_bindings_shader_defs = Vec::with_capacity(1);

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            let render_device = render_app.world().resource::<RenderDevice>();
            let batched_instance_buffer =
                BatchedInstanceBuffer::<Mesh2dUniform>::new(render_device);

            if let Some(per_object_buffer_batch_size) =
                GpuArrayBuffer::<Mesh2dUniform>::batch_size(render_device)
            {
                mesh_bindings_shader_defs.push(ShaderDefVal::UInt(
                    "PER_OBJECT_BUFFER_BATCH_SIZE".into(),
                    per_object_buffer_batch_size,
                ));
            }

            render_app
                .insert_resource(batched_instance_buffer)
                .init_resource::<Mesh2dPipeline>()
                .init_resource::<ViewKeyCache>()
                .init_resource::<ViewSpecializationTicks>()
                .add_systems(
                    Render,
                    check_views_need_specialization.in_set(RenderSet::PrepareAssets),
                );
        }

        // Load the mesh_bindings shader module here as it depends on runtime information about
        // whether storage buffers are supported, or the maximum uniform buffer binding size.
        load_internal_asset!(
            app,
            MESH2D_BINDINGS_HANDLE,
            "shaders/mesh2d_bindings.wgsl",
            Shader::from_wgsl_with_defs,
            mesh_bindings_shader_defs
        );
    }
}

// NOTE: These must match the bit flags in bevy_render_2d/src/mesh/shaders/mesh2d.wgsl!
bitflags::bitflags! {
    #[repr(transparent)]
    pub struct MeshFlags: u32 {
        const NONE                       = 0;
        const UNINITIALIZED              = 0xFFFF;
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

pub fn check_views_need_specialization(
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

pub const fn tonemapping_pipeline_key(tonemapping: Tonemapping) -> Mesh2dPipelineKey {
    match tonemapping {
        Tonemapping::None => Mesh2dPipelineKey::TONEMAP_METHOD_NONE,
        Tonemapping::Reinhard => Mesh2dPipelineKey::TONEMAP_METHOD_REINHARD,
        Tonemapping::ReinhardLuminance => Mesh2dPipelineKey::TONEMAP_METHOD_REINHARD_LUMINANCE,
        Tonemapping::AcesFitted => Mesh2dPipelineKey::TONEMAP_METHOD_ACES_FITTED,
        Tonemapping::AgX => Mesh2dPipelineKey::TONEMAP_METHOD_AGX,
        Tonemapping::SomewhatBoringDisplayTransform => {
            Mesh2dPipelineKey::TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM
        }
        Tonemapping::TonyMcMapface => Mesh2dPipelineKey::TONEMAP_METHOD_TONY_MC_MAPFACE,
        Tonemapping::BlenderFilmic => Mesh2dPipelineKey::TONEMAP_METHOD_BLENDER_FILMIC,
    }
}

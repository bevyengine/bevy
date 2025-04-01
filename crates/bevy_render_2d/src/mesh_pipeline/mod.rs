//! Provides functionality for rendering meshes in 2d

pub mod commands;
pub mod key;
pub mod pipeline;
pub mod render;
mod systems;

use bevy_app::Plugin;
use bevy_asset::{load_internal_asset, weak_handle, Handle};
use bevy_core_pipeline::core_2d::{AlphaMask2d, Opaque2d, Transparent2d};
use bevy_ecs::prelude::*;
use bevy_render::{
    batching::no_gpu_preprocessing::{
        self, batch_and_prepare_binned_render_phase, batch_and_prepare_sorted_render_phase,
        write_batched_instance_buffer, BatchedInstanceBuffer,
    },
    render_phase::sweep_old_entities,
    render_resource::*,
    renderer::RenderDevice,
    ExtractSchedule, Render, RenderApp,
    RenderSet::{self},
};

use pipeline::Mesh2dPipeline;
use render::{Mesh2dUniform, RenderMesh2dInstances, ViewKeyCache, ViewSpecializationTicks};
use systems::{
    check_views_need_specialization, extract_mesh2d, prepare_mesh2d_bind_group,
    prepare_mesh2d_view_bind_groups,
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

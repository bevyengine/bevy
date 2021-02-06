use crate::{
    pipeline::{
    BlendFactor, BlendOperation, ColorWrite,
    CompareFunction, CullMode, FrontFace, PipelineDescriptor,
    PolygonMode, RenderPipeline,
    },
    prelude::*,
    shader::{Shader, ShaderStage, ShaderStages},
    texture::TextureFormat,
};
use bevy_app::prelude::*;
use bevy_asset::{Assets, Handle, HandleUntyped};
use bevy_ecs::{Entity, World, ResMut, Res, Query};
use bevy_reflect::TypeUuid;
use crate::draw::DrawContext;
use crate::renderer::RenderResourceBindings;
use crate::mesh::Indices;
use crate::pipeline::{PipelineSpecialization, VertexBufferLayout};
use bevy_ecs::IntoSystem;
use bevy_utils::HashSet;

mod pipeline;

pub const WIREFRAME_PIPELINE_HANDLE: HandleUntyped =
HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 0x137c75ab7e9ad7f5);

#[derive(Debug, Default)]
pub struct WireframePlugin;

impl Plugin for WireframePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_system_to_stage(crate::stage::DRAW, draw_wireframes_system.system());
        let resources = app.resources();
        let mut shaders = resources.get_mut::<Assets<Shader>>().unwrap();
        let mut pipelines = resources.get_mut::<Assets<PipelineDescriptor>>().unwrap();
        pipelines.set(
            WIREFRAME_PIPELINE_HANDLE,
            pipeline::build_wireframe_pipeline(&mut shaders),
        );
    }
}



pub fn draw_wireframes_system(
    mut draw_context: DrawContext,
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    msaa: Res<Msaa>,
    meshes: Res<Assets<Mesh>>,
    mut query: Query<(&mut Draw, &mut RenderPipelines, &Handle<Mesh>, &Visible)>,
) {
    for (mut draw, mut render_pipelines, mesh_handle, visible) in query.iter_mut() {
        if !visible.is_visible {
            continue;
        }

        // don't render if the mesh isn't loaded yet
        let mesh = if let Some(mesh) = meshes.get(mesh_handle) {
            mesh
        } else {
            continue;
        };

        let index_range = match mesh.indices() {
            Some(Indices::U32(indices)) => Some(0..indices.len() as u32),
            Some(Indices::U16(indices)) => Some(0..indices.len() as u32),
            None => None,
        };

        let mut render_pipeline = RenderPipeline::specialized(
            WIREFRAME_PIPELINE_HANDLE.typed(),
            PipelineSpecialization {
                sample_count: msaa.samples,
                strip_index_format: None,
                shader_specialization: Default::default(),
                primitive_topology: mesh.primitive_topology(),
                dynamic_bindings: Default::default(),
                vertex_buffer_layout: mesh.get_vertex_buffer_layout(),
            },
        );
        if render_pipeline.dynamic_bindings_generation
            != render_pipelines.bindings.dynamic_bindings_generation()
        {
            render_pipeline.specialization.dynamic_bindings = render_pipelines
                .bindings
                .iter_dynamic_bindings()
                .map(|name| name.to_string())
                .collect::<HashSet<String>>();
            render_pipeline.dynamic_bindings_generation =
                render_pipelines.bindings.dynamic_bindings_generation();
            for (handle, _) in render_pipelines.bindings.iter_assets() {
                if let Some(bindings) = draw_context
                    .asset_render_resource_bindings
                    .get_untyped(handle)
                {
                    for binding in bindings.iter_dynamic_bindings() {
                        render_pipeline
                            .specialization
                            .dynamic_bindings
                            .insert(binding.to_string());
                    }
                }
            }
        }

        let render_resource_bindings = &mut [
            &mut render_pipelines.bindings,
            &mut render_resource_bindings,
        ];
        draw_context
            .set_pipeline(
                &mut draw,
                &render_pipeline.pipeline,
                &render_pipeline.specialization,
            )
            .unwrap();
        draw_context
            .set_bind_groups_from_bindings(&mut draw, render_resource_bindings)
            .unwrap();
        draw_context
            .set_vertex_buffers_from_bindings(&mut draw, &[&render_pipelines.bindings])
            .unwrap();
        if let Some(indices) = index_range.clone() {
            draw.draw_indexed(indices, 0, 0..1);
        } else {
            draw.draw(0..mesh.count_vertices() as u32, 0..1)
        }
    }
}

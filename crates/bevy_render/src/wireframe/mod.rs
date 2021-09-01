use crate::{
    draw::DrawContext,
    mesh::Indices,
    pipeline::{PipelineDescriptor, PipelineSpecialization, RenderPipeline},
    prelude::*,
    shader::Shader,
};
use bevy_app::prelude::*;
use bevy_asset::{Assets, Handle, HandleUntyped};
use bevy_ecs::{
    query::{QueryState, With},
    reflect::ReflectComponent,
    system::{QuerySet, Res},
    world::Mut,
};
use bevy_reflect::{Reflect, TypeUuid};
use bevy_utils::HashSet;

mod pipeline;

pub const WIREFRAME_PIPELINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 0x137c75ab7e9ad7f5);

#[derive(Debug, Default)]
pub struct WireframePlugin;

impl Plugin for WireframePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WireframeConfig>()
            .add_system_to_stage(crate::RenderStage::Draw, draw_wireframes_system);
        let world = app.world.cell();
        let mut shaders = world.get_resource_mut::<Assets<Shader>>().unwrap();
        let mut pipelines = world
            .get_resource_mut::<Assets<PipelineDescriptor>>()
            .unwrap();
        pipelines.set_untracked(
            WIREFRAME_PIPELINE_HANDLE,
            pipeline::build_wireframe_pipeline(&mut shaders),
        );
    }
}

#[derive(Debug, Clone, Reflect, Default)]
#[reflect(Component)]
pub struct Wireframe;

#[derive(Debug, Clone)]
pub struct WireframeConfig {
    pub global: bool,
}

impl Default for WireframeConfig {
    fn default() -> Self {
        WireframeConfig { global: false }
    }
}

#[allow(clippy::type_complexity)]
pub fn draw_wireframes_system(
    mut draw_context: DrawContext,
    msaa: Res<Msaa>,
    meshes: Res<Assets<Mesh>>,
    wireframe_config: Res<WireframeConfig>,
    mut query: QuerySet<(
        QueryState<(&mut Draw, &mut RenderPipelines, &Handle<Mesh>, &Visible)>,
        QueryState<(&mut Draw, &mut RenderPipelines, &Handle<Mesh>, &Visible), With<Wireframe>>,
    )>,
) {
    let iterator = |(mut draw, mut render_pipelines, mesh_handle, visible): (
        Mut<Draw>,
        Mut<RenderPipelines>,
        &Handle<Mesh>,
        &Visible,
    )| {
        if !visible.is_visible {
            return;
        }

        // don't render if the mesh isn't loaded yet
        let mesh = if let Some(mesh) = meshes.get(mesh_handle) {
            mesh
        } else {
            return;
        };

        let mut render_pipeline = RenderPipeline::specialized(
            WIREFRAME_PIPELINE_HANDLE.typed(),
            PipelineSpecialization {
                sample_count: msaa.samples,
                strip_index_format: None,
                shader_specialization: Default::default(),
                primitive_topology: mesh.primitive_topology(),
                dynamic_bindings: render_pipelines
                    .bindings
                    .iter_dynamic_bindings()
                    .map(|name| name.to_string())
                    .collect::<HashSet<String>>(),
                vertex_buffer_layout: mesh.get_vertex_buffer_layout(),
            },
        );
        render_pipeline.dynamic_bindings_generation =
            render_pipelines.bindings.dynamic_bindings_generation();

        draw_context
            .set_pipeline(
                &mut draw,
                &render_pipeline.pipeline,
                &render_pipeline.specialization,
            )
            .unwrap();
        draw_context
            .set_bind_groups_from_bindings(&mut draw, &mut [&mut render_pipelines.bindings])
            .unwrap();
        draw_context
            .set_vertex_buffers_from_bindings(&mut draw, &[&render_pipelines.bindings])
            .unwrap();

        match mesh.indices() {
            Some(Indices::U32(indices)) => draw.draw_indexed(0..indices.len() as u32, 0, 0..1),
            Some(Indices::U16(indices)) => draw.draw_indexed(0..indices.len() as u32, 0, 0..1),
            None => draw.draw(0..mesh.count_vertices() as u32, 0..1),
        };
    };

    if wireframe_config.global {
        query.q0().iter_mut().for_each(iterator);
    } else {
        query.q1().iter_mut().for_each(iterator);
    }
}

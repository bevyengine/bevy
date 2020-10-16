use super::{PipelineDescriptor, PipelineSpecialization};
use crate::{
    draw::{Draw, DrawContext},
    mesh::{Indices, Mesh},
    prelude::Msaa,
    renderer::RenderResourceBindings,
};
use bevy_asset::{Assets, Handle};
use bevy_ecs::{Query, Res, ResMut};
use bevy_property::Properties;

#[derive(Debug, Properties, Default, Clone)]
#[non_exhaustive]
pub struct RenderPipeline {
    pub pipeline: Handle<PipelineDescriptor>,
    pub specialization: PipelineSpecialization,
}

impl RenderPipeline {
    pub fn new(pipeline: Handle<PipelineDescriptor>) -> Self {
        RenderPipeline {
            pipeline,
            ..Default::default()
        }
    }

    pub fn specialized(
        pipeline: Handle<PipelineDescriptor>,
        specialization: PipelineSpecialization,
    ) -> Self {
        RenderPipeline {
            pipeline,
            specialization,
        }
    }
}

#[derive(Debug, Properties, Clone)]
pub struct RenderPipelines {
    pub pipelines: Vec<RenderPipeline>,
    #[property(ignore)]
    pub bindings: RenderResourceBindings,
}

impl RenderPipelines {
    pub fn from_pipelines(pipelines: Vec<RenderPipeline>) -> Self {
        Self {
            pipelines,
            ..Default::default()
        }
    }

    pub fn from_handles<'a, T: IntoIterator<Item = &'a Handle<PipelineDescriptor>>>(
        handles: T,
    ) -> Self {
        RenderPipelines {
            pipelines: handles
                .into_iter()
                .map(|pipeline| RenderPipeline::new(pipeline.clone_weak()))
                .collect::<Vec<RenderPipeline>>(),
            ..Default::default()
        }
    }
}

impl Default for RenderPipelines {
    fn default() -> Self {
        Self {
            bindings: Default::default(),
            pipelines: vec![RenderPipeline::default()],
        }
    }
}

pub fn draw_render_pipelines_system(
    mut draw_context: DrawContext,
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    msaa: Res<Msaa>,
    meshes: Res<Assets<Mesh>>,
    mut query: Query<(&mut Draw, &mut RenderPipelines, &Handle<Mesh>)>,
) {
    for (mut draw, mut render_pipelines, mesh_handle) in query.iter_mut() {
        if !draw.is_visible {
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

        let render_pipelines = &mut *render_pipelines;
        for pipeline in render_pipelines.pipelines.iter_mut() {
            pipeline.specialization.sample_count = msaa.samples;
            // TODO: move these to mesh.rs?
        }

        for render_pipeline in render_pipelines.pipelines.iter_mut() {
            let render_resource_bindings = &mut [
                &mut render_pipelines.bindings,
                &mut render_resource_bindings,
            ];
            draw_context
                .set_pipeline(
                    &mut draw,
                    &render_pipeline.pipeline,
                    &mut render_pipeline.specialization,
                    render_resource_bindings,
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
            }
        }
    }
}

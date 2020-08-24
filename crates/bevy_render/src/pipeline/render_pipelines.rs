use super::{PipelineDescriptor, PipelineSpecialization};
use crate::{
    draw::{Draw, DrawContext},
    prelude::Msaa,
    renderer::RenderResourceBindings,
};
use bevy_asset::Handle;
use bevy_ecs::{Query, Res, ResMut};
use bevy_property::Properties;

#[derive(Properties, Default, Clone)]
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

#[derive(Properties)]
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
                .map(|pipeline| RenderPipeline::new(*pipeline))
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
    mut query: Query<(&mut Draw, &mut RenderPipelines)>,
) {
    for (mut draw, mut render_pipelines) in &mut query.iter() {
        if !draw.is_visible {
            continue;
        }
        let render_pipelines = &mut *render_pipelines;
        for pipeline in render_pipelines.pipelines.iter_mut() {
            pipeline.specialization.sample_count = msaa.samples;
        }

        for render_pipeline in render_pipelines.pipelines.iter() {
            draw_context
                .set_pipeline(
                    &mut draw,
                    render_pipeline.pipeline,
                    &render_pipeline.specialization,
                )
                .unwrap();
            draw_context
                .set_bind_groups_from_bindings(
                    &mut draw,
                    &mut [
                        &mut render_pipelines.bindings,
                        &mut render_resource_bindings,
                    ],
                )
                .unwrap();
            let indices = draw_context
                .set_vertex_buffers_from_bindings(&mut draw, &[&render_pipelines.bindings])
                .unwrap();
            if let Some(indices) = indices {
                draw.draw_indexed(indices, 0, 0..1);
            }
        }
    }
}

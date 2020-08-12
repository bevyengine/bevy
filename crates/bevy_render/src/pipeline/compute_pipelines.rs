use super::{ComputePipelineDescriptor, PipelineSpecialization};
use crate::{
    renderer::RenderResourceBindings, dispatch::{ComputeContext, Dispatch},
};
use bevy_asset::Handle;
use bevy_ecs::{Query, ResMut};
use bevy_property::Properties;
#[derive(Properties, Default, Clone)]
pub struct ComputePipeline {
    pub pipeline: Handle<ComputePipelineDescriptor>,
    pub specialization: PipelineSpecialization,
}

impl ComputePipeline {
    pub fn new(pipeline: Handle<ComputePipelineDescriptor>) -> Self {
        ComputePipeline {
            pipeline,
            ..Default::default()
        }
    }

    pub fn specialized(
        pipeline: Handle<ComputePipelineDescriptor>,
        specialization: PipelineSpecialization,
    ) -> Self {
        ComputePipeline {
            pipeline,
            specialization,
            ..Default::default()
        }
    }
}

#[derive(Properties)]
pub struct ComputePipelines {
    pub pipelines: Vec<ComputePipeline>,
    #[property(ignore)]
    pub bindings: RenderResourceBindings,
}

impl ComputePipelines {
    pub fn from_pipelines(pipelines: Vec<ComputePipeline>) -> Self {
        Self {
            pipelines,
            ..Default::default()
        }
    }

    pub fn from_handles<'a, T: IntoIterator<Item = &'a Handle<ComputePipelineDescriptor>>>(
        handles: T,
    ) -> Self {
        ComputePipelines {
            pipelines: handles
                .into_iter()
                .map(|pipeline| ComputePipeline::new(*pipeline))
                .collect::<Vec<ComputePipeline>>(),
            ..Default::default()
        }
    }
}

impl Default for ComputePipelines {
    fn default() -> Self {
        Self {
            bindings: Default::default(),
            pipelines: vec![ComputePipeline::default()],
        }
    }
}

pub fn draw_compute_pipelines_system(
    mut _draw_context: ComputeContext,
    mut _render_resource_bindings: ResMut<RenderResourceBindings>,
    mut query: Query<(&mut Dispatch, &mut ComputePipelines)>,
) {
    for (mut _dispatch, mut compute_pipelines) in &mut query.iter() {
        let compute_pipelines = &mut *compute_pipelines;

        for _compute_pipeline in compute_pipelines.pipelines.iter() {
            // TODO: I think we need a compute_context here or allow draw_context to accept either a compute pipeline or a render pipeline..

            // draw_context
            //     .set_pipeline(
            //         &mut dispatch,
            //         compute_pipeline.pipeline,
            //         &compute_pipeline.specialization,
            //     )
            //     .unwrap();
            // draw_context
            //     .set_bind_groups_from_bindings(
            //         &mut dispatch,
            //         &mut [
            //             &mut compute_pipelines.bindings,
            //             &mut render_resource_bindings,
            //         ],
            //     )
            //     .unwrap();

            todo!();
        }
    }
}

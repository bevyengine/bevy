use super::{ComputePipelineDescriptor, ComputePipelineSpecialization};
use crate::{
    dispatch::{Dispatch, DispatchContext},
    renderer::RenderResourceBindings,
};
use bevy_asset::Handle;
use bevy_ecs::{Query, ResMut};
use bevy_property::Properties;

#[derive(Properties, Default, Clone)]
pub struct ComputePipeline {
    pub pipeline: Handle<ComputePipelineDescriptor>,
    pub specialization: ComputePipelineSpecialization,
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
        specialization: ComputePipelineSpecialization,
    ) -> Self {
        ComputePipeline {
            pipeline,
            specialization,
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

pub fn dispatch_compute_pipelines_system(
    mut dispatch_context: DispatchContext,
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    mut query: Query<(&mut Dispatch, &mut ComputePipelines)>,
) {
    for (mut dispatch, mut render_pipelines) in &mut query.iter() {
        let render_pipelines = &mut *render_pipelines;

        for render_pipeline in render_pipelines.pipelines.iter() {
            dispatch_context
                .set_pipeline(
                    &mut dispatch,
                    render_pipeline.pipeline,
                    &render_pipeline.specialization,
                )
                .unwrap();
            dispatch_context
                .set_bind_groups_from_bindings(
                    &mut dispatch,
                    &mut [
                        &mut render_pipelines.bindings,
                        &mut render_resource_bindings,
                    ],
                )
                .unwrap();
            dispatch.dispatch();
        }
    }
}

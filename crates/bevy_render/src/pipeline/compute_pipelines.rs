use super::{ComputePipelineDescriptor, ComputePipelineSpecialization};
use crate::{
    renderer::RenderResourceBindings, dispatch::{ComputeContext, Dispatch},
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

// TODO: Not sure this currently makes sense.
pub fn dispatch_compute_pipelines_system(
    mut compute_context: ComputeContext,
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    mut query: Query<(&mut Dispatch, &mut ComputePipelines)>,
) {
    for (mut dispatch, mut compute_pipelines) in &mut query.iter() {
        let compute_pipelines = &mut *compute_pipelines;

        for compute_pipeline in compute_pipelines.pipelines.iter() {
            
            compute_context
                .set_pipeline(
                    &mut dispatch,
                    compute_pipeline.pipeline,
                    &compute_pipeline.specialization,
                )
                .unwrap();
            compute_context
                .set_bind_groups_from_bindings(
                    &mut dispatch,
                    &mut [
                        &mut compute_pipelines.bindings,
                        &mut render_resource_bindings,
                    ],
                )
                .unwrap();

        }
    }
}

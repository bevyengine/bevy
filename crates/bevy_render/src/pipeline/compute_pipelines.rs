use super::{ComputePipelineDescriptor, ComputePipelineSpecialization};
use crate::{
    renderer::RenderResourceBindings,
};
use bevy_asset::Handle;
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

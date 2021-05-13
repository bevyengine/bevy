use super::{ComputePipelineSpecialization, PipelineLayout};
use crate::{
    renderer::RenderResourceBindings,
    shader::{ComputeShaderStages},
};
use bevy_asset::{Handle};
use bevy_ecs::{
    reflect::ReflectComponent,
};
use bevy_reflect::{Reflect, TypeUuid};

/// Compute pipeline descriptor
#[derive(Clone, Debug, TypeUuid)]
#[uuid = "83b4751d-adf6-49f9-8765-3772ac8ac3a2"]
pub struct ComputePipelineDescriptor {
    pub name: Option<String>,
    pub layout: Option<PipelineLayout>,
    pub shader_stages: ComputeShaderStages,
}

impl ComputePipelineDescriptor {
    pub fn new(shader_stages: ComputeShaderStages) -> Self {
        ComputePipelineDescriptor {
            name: None,
            layout: None,
            shader_stages,
        }
    }

    pub fn get_layout(&self) -> Option<&PipelineLayout> {
        self.layout.as_ref()
    }

    pub fn get_layout_mut(&mut self) -> Option<&mut PipelineLayout> {
        self.layout.as_mut()
    }
}

#[derive(Debug, Default, Clone, Reflect)]
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

#[derive(Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct ComputePipelines {
    pub pipelines: Vec<ComputePipeline>,
    #[reflect(ignore)]
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
                .map(|pipeline| ComputePipeline::new(pipeline.clone_weak()))
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

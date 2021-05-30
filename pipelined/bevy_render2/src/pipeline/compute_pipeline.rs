use super::PipelineLayout;
use crate::shader::ComputeShaderStages;
use bevy_reflect::TypeUuid;

#[derive(Clone, Debug, TypeUuid)]
#[uuid = "359c1fff-0c86-4fbb-8b66-4e2af422a2f1"]
pub struct ComputePipelineDescriptor {
    pub name: Option<String>,
    pub layout: PipelineLayout,
    pub shader_stages: ComputeShaderStages,
}

impl ComputePipelineDescriptor {
    pub fn new(shader_stages: ComputeShaderStages, layout: PipelineLayout) -> Self {
        ComputePipelineDescriptor {
            name: None,
            layout,
            shader_stages,
        }
    }

    pub fn default_config(shader_stages: ComputeShaderStages, layout: PipelineLayout) -> Self {
        ComputePipelineDescriptor {
            name: None,
            layout,
            shader_stages,
        }
    }
}

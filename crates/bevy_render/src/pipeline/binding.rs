use super::UniformProperty;
use crate::texture::{TextureComponentType, TextureFormat, TextureViewDimension};

bitflags::bitflags! {
    pub struct BindingShaderStage: u32 {
        const VERTEX = 1;
        const FRAGMENT = 2;
        const COMPUTE = 4;
    }
}

#[derive(Hash, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct BindingDescriptor {
    pub name: String,
    pub index: u32,
    pub bind_type: BindType,
    pub shader_stage: BindingShaderStage,
}

#[derive(Hash, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum BindType {
    Uniform {
        dynamic: bool,
        property: UniformProperty,
    },
    StorageBuffer {
        dynamic: bool,
        readonly: bool,
    },
    Sampler {
        comparison: bool,
    },
    SampledTexture {
        multisampled: bool,
        dimension: TextureViewDimension,
        component_type: TextureComponentType,
    },
    StorageTexture {
        dimension: TextureViewDimension,
        format: TextureFormat,
        readonly: bool,
    },
}

impl BindType {
    pub fn get_uniform_size(&self) -> Option<u64> {
        match self {
            BindType::Uniform { property, .. } => Some(property.get_size()),
            _ => None,
        }
    }
}

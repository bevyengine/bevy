use super::UniformProperty;
use crate::texture::{
    StorageTextureAccess, TextureFormat, TextureSampleType, TextureViewDimension,
};

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
        has_dynamic_offset: bool,
        property: UniformProperty,
    },
    StorageBuffer {
        has_dynamic_offset: bool,
        readonly: bool,
    },
    Sampler {
        /// The sampling result is produced based on more than a single color sample from a
        /// texture, e.g. when bilinear interpolation is enabled.
        ///
        /// A filtering sampler can only be used with a filterable texture.
        filtering: bool,
        /// Use as a comparison sampler instead of a normal sampler.
        /// For more info take a look at the analogous functionality in OpenGL: <https://www.khronos.org/opengl/wiki/Sampler_Object#Comparison_mode>.
        comparison: bool,
    },
    Texture {
        multisampled: bool,
        view_dimension: TextureViewDimension,
        sample_type: TextureSampleType,
    },
    StorageTexture {
        /// Allowed access to this texture.
        access: StorageTextureAccess,
        /// Format of the texture.
        format: TextureFormat,
        /// Dimension of the texture view that is going to be sampled.
        view_dimension: TextureViewDimension,
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

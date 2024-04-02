use super::{
    node::RenderGraphNode,
    resource::{RenderGraphResource, RenderGraphResourceUsage, RenderGraphResourceUsageType},
};
use crate::{prelude::Shader, render_resource::ShaderDefVal};
use bevy_asset::Handle;
use bevy_math::UVec3;

pub struct ComputePass {
    pub(crate) label: &'static str,
    pub(crate) shader: Handle<Shader>,
    pub(crate) shader_defs: Vec<ShaderDefVal>,
    pub(crate) resource_usages: Vec<RenderGraphResourceUsage>,
    pub(crate) dispatch_size: UVec3,
}

impl ComputePass {
    pub fn new(label: &'static str, shader: Handle<Shader>) -> Self {
        Self {
            label,
            shader,
            shader_defs: Vec::new(),
            resource_usages: Vec::new(),
            dispatch_size: UVec3::ZERO,
        }
    }

    pub fn shader_def(mut self, name: &'static str, condition: bool) -> Self {
        self.shader_defs
            .push(ShaderDefVal::Bool(name.to_owned(), condition));
        self
    }

    pub fn shader_def_val(mut self, name: &'static str, value: u32) -> Self {
        self.shader_defs
            .push(ShaderDefVal::UInt(name.to_owned(), value));
        self
    }

    pub fn read_texture(mut self, texture: &RenderGraphResource) -> Self {
        self.resource_usages.push(RenderGraphResourceUsage {
            resource: texture.clone(),
            usage_type: RenderGraphResourceUsageType::ReadTexture,
        });
        self
    }

    pub fn write_texture(mut self, texture: &mut RenderGraphResource) -> Self {
        self.resource_usages.push(RenderGraphResourceUsage {
            resource: texture.clone(),
            usage_type: RenderGraphResourceUsageType::WriteTexture,
        });
        texture.generation += 1;
        self
    }

    pub fn read_write_texture(mut self, texture: &mut RenderGraphResource) -> Self {
        self.resource_usages.push(RenderGraphResourceUsage {
            resource: texture.clone(),
            usage_type: RenderGraphResourceUsageType::ReadWriteTexture,
        });
        texture.generation += 1;
        self
    }

    pub fn dispatch(mut self, size: UVec3) -> Self {
        self.dispatch_size = size;
        self
    }
}

impl Into<RenderGraphNode> for ComputePass {
    fn into(self) -> RenderGraphNode {
        todo!()
    }
}

use super::resource::RenderGraphResourceUsage;
use crate::{
    prelude::Shader,
    render_resource::{
        BindGroup, BindGroupLayout, CachedComputePipelineId, ComputePipeline, ShaderDefVal,
    },
    renderer::{RenderDevice, RenderQueue},
};
use bevy_asset::Handle;

type NodeRunner =
    dyn FnOnce(&RenderDevice, &RenderQueue, &ComputePipeline, &BindGroup) + Send + Sync + 'static;

pub struct RenderGraphNode {
    pub(crate) label: &'static str,
    pub(crate) shader: Handle<Shader>,
    pub(crate) shader_defs: Vec<ShaderDefVal>,
    pub(crate) resource_usages: Box<[RenderGraphResourceUsage]>,
    pub(crate) runner: Box<NodeRunner>,
    pub(crate) bind_group_layout: Option<BindGroupLayout>,
    pub(crate) pipeline: Option<CachedComputePipelineId>,
    pub(crate) bind_group: Option<BindGroup>,
}

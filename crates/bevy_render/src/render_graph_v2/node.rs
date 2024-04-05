use super::resource::RenderGraphResourceUsage;
use crate::{
    prelude::Shader,
    render_resource::{BindGroupLayout, CachedComputePipelineId, ShaderDefVal},
    renderer::{RenderDevice, RenderQueue},
};
use bevy_asset::Handle;

pub struct RenderGraphNode {
    pub(crate) label: &'static str,
    pub(crate) shader: Handle<Shader>,
    pub(crate) shader_defs: Vec<ShaderDefVal>,
    pub(crate) resource_usages: Box<[RenderGraphResourceUsage]>,
    pub(crate) runner: Box<dyn FnOnce(&RenderDevice, &RenderQueue) + Send + Sync + 'static>,
    pub(crate) bind_group_layout: Option<BindGroupLayout>,
    pub(crate) pipeline: Option<CachedComputePipelineId>,
}

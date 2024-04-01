use super::resource::RenderGraphResourceUsage;
use crate::{
    render_resource::BindGroupLayout,
    renderer::{RenderDevice, RenderQueue},
};

pub struct RenderGraphNode {
    pub(crate) label: &'static str,
    pub(crate) resource_usages: Box<[RenderGraphResourceUsage]>,
    pub(crate) runner: Box<dyn FnOnce(&RenderDevice, &RenderQueue) + Send + Sync + 'static>,
    pub(crate) bind_group_layout: Option<BindGroupLayout>,
}

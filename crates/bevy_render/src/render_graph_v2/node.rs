use super::resource::RenderGraphResource;
use crate::renderer::{RenderDevice, RenderQueue};

pub struct RenderGraphNode {
    resources: Box<[RenderGraphResource]>,
    runner: Box<dyn FnOnce(&RenderDevice, &RenderQueue) + Send + Sync + 'static>,
}

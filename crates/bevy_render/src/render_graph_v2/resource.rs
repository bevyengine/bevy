/// Handle to a resource for use within a [`super::RenderGraph`].
#[derive(Clone)] // TODO: Should this be Copy?
pub struct RenderGraphResource {
    /// Uniquely identifies a resource within the render graph.
    pub(crate) id: RenderGraphResourceId,
    /// Counter starting at 0 that gets incremented every time the resource is modified.
    pub(crate) generation: u16,
}

/// Uniquely identifies a resource within a [`super::RenderGraph`].
pub type RenderGraphResourceId = u16;

/// Usage of a [`RenderGraphResource`] within a [`RenderGraphNode`].
pub struct RenderGraphResourceUsage {
    /// The resource used by the node.
    pub resource: RenderGraphResource,
    /// How the resource is used by the node.
    pub usage_type: RenderGraphResourceUsageType,
}

/// Type of resource usage for a [`RenderGraphResourceUsage`].
pub enum RenderGraphResourceUsageType {
    /// Corresponds to [`wgpu::BindingType::Texture`].
    ReadTexture,
    /// Corresponds to [`wgpu::BindingType::StorageTexture`] with [`wgpu::StorageTextureAccess::WriteOnly`].
    WriteTexture,
    /// Corresponds to [`wgpu::BindingType::StorageTexture`] with [`wgpu::StorageTextureAccess::ReadWrite`].
    ReadWriteTexture,
}

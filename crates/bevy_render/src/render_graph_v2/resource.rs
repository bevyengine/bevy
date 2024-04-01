/// Handle to a resource for use within a [`super::RenderGraph`].
pub struct RenderGraphResource {
    /// Unique ID within a render graph.
    pub(crate) id: RenderGraphResourceId,
    /// Counter starting at 0 that gets increment every time the resource is used.
    pub(crate) generation: u16,
}

impl RenderGraphResource {
    /// Increment this resource's generation and return a new copy.
    pub(crate) fn increment(&mut self) -> Self {
        self.generation += 1;
        Self {
            id: self.id,
            generation: self.generation,
        }
    }
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

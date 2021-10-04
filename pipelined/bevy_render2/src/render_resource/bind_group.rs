use bevy_reflect::Uuid;
use std::sync::Arc;

/// A [`BindGroup`] identifier.
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct BindGroupId(Uuid);

/// Bind groups are responsible for binding render resources (e.g. buffers, textures, samplers)
/// to a [`TrackedRenderPass`](crate::render_phase::TrackedRenderPass).
/// This makes them accessible in the pipeline (shaders) as uniforms.
///
/// May be converted from and dereferences to a wgpu [`BindGroup`](wgpu::BindGroup).
/// Can be created via [`RenderDevice::create_bind_group`](crate::renderer::RenderDevice::create_bind_group).
#[derive(Clone, Debug)]
pub struct BindGroup {
    id: BindGroupId,
    value: Arc<wgpu::BindGroup>,
}

impl BindGroup {
    /// Returns the [`BindGroupId`].
    #[inline]
    pub fn id(&self) -> BindGroupId {
        self.id
    }

    /// Returns the wgpu [`BindGroup`](wgpu::BindGroup)
    #[inline]
    pub fn value(&self) -> &wgpu::BindGroup {
        &self.value
    }
}

impl From<wgpu::BindGroup> for BindGroup {
    fn from(value: wgpu::BindGroup) -> Self {
        BindGroup {
            id: BindGroupId(Uuid::new_v4()),
            value: Arc::new(value),
        }
    }
}

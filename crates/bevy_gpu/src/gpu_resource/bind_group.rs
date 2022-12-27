use crate::{define_atomic_id, gpu_resource::resource_macros::*};
use std::ops::Deref;

define_atomic_id!(BindGroupId);
gpu_resource_wrapper!(ErasedBindGroup, wgpu::BindGroup);

/// Bind groups are responsible for binding render resources (e.g. buffers, textures, samplers)
/// to a render pass.
/// This makes them accessible in the pipeline (shaders) as uniforms.
///
/// May be converted from and dereferences to a wgpu [`BindGroup`](wgpu::BindGroup).
/// Can be created via [`GPUDevice::create_bind_group`](crate::GpuDevice::create_bind_group).
#[derive(Clone, Debug)]
pub struct BindGroup {
    id: BindGroupId,
    value: ErasedBindGroup,
}

impl BindGroup {
    /// Returns the [`BindGroupId`].
    #[inline]
    pub fn id(&self) -> BindGroupId {
        self.id
    }
}

impl From<wgpu::BindGroup> for BindGroup {
    fn from(value: wgpu::BindGroup) -> Self {
        BindGroup {
            id: BindGroupId::new(),
            value: ErasedBindGroup::new(value),
        }
    }
}

impl Deref for BindGroup {
    type Target = wgpu::BindGroup;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

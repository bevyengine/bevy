use crate::renderer::wgpu_wrapper;
use bevy_utils::define_atomic_id;
use core::ops::Deref;

define_atomic_id!(BindGroupLayoutId);

wgpu_wrapper! {
    #[derive(Clone, Debug)]
    struct WgpuBindGroupLayout(wgpu::BindGroupLayout);
}

/// Bind group layouts define the interface of resources (e.g. buffers, textures, samplers)
/// for a shader. The actual resource binding is done via a [`BindGroup`](super::BindGroup).
///
/// This is a lightweight thread-safe wrapper around wgpu's own [`BindGroupLayout`](wgpu::BindGroupLayout),
/// which can be cloned as needed to workaround lifetime management issues. It may be converted
/// from and dereferences to wgpu's [`BindGroupLayout`](wgpu::BindGroupLayout).
///
/// Can be created via [`RenderDevice::create_bind_group_layout`](crate::renderer::RenderDevice::create_bind_group_layout).
#[derive(Clone, Debug)]
pub struct BindGroupLayout {
    id: BindGroupLayoutId,
    value: WgpuBindGroupLayout,
}

impl PartialEq for BindGroupLayout {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for BindGroupLayout {}

impl core::hash::Hash for BindGroupLayout {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.id.0.hash(state);
    }
}

impl BindGroupLayout {
    /// Returns the [`BindGroupLayoutId`] representing the unique ID of the bind group layout.
    #[inline]
    pub fn id(&self) -> BindGroupLayoutId {
        self.id
    }

    #[inline]
    pub fn value(&self) -> &wgpu::BindGroupLayout {
        &self.value
    }
}

impl From<wgpu::BindGroupLayout> for BindGroupLayout {
    fn from(value: wgpu::BindGroupLayout) -> Self {
        BindGroupLayout {
            id: BindGroupLayoutId::new(),
            value: WgpuBindGroupLayout::new(value),
        }
    }
}

impl Deref for BindGroupLayout {
    type Target = wgpu::BindGroupLayout;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

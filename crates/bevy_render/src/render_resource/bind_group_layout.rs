use crate::renderer::{impl_eq_ord_hash_wrapper, wgpu_wrapper};
use core::ops::Deref;

wgpu_wrapper! {
    #[derive(Clone, Debug)]
    struct WgpuBindGroupLayout(wgpu::BindGroupLayout);
}

impl_eq_ord_hash_wrapper!(WgpuBindGroupLayout);

/// An opaque identifier for a [`BindGroupLayout`], backed by the wrapped wgpu [`BindGroupLayout`](wgpu::BindGroupLayout).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BindGroupLayoutId(WgpuBindGroupLayout);

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
    value: WgpuBindGroupLayout,
}

impl PartialEq for BindGroupLayout {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl Eq for BindGroupLayout {}

impl core::hash::Hash for BindGroupLayout {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl BindGroupLayout {
    /// Returns the [`BindGroupLayoutId`] representing the unique ID of the bind group layout.
    #[inline]
    pub fn id(&self) -> BindGroupLayoutId {
        BindGroupLayoutId(self.value.clone())
    }

    #[inline]
    pub fn value(&self) -> &wgpu::BindGroupLayout {
        &self.value
    }
}

impl From<wgpu::BindGroupLayout> for BindGroupLayout {
    fn from(value: wgpu::BindGroupLayout) -> Self {
        BindGroupLayout {
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

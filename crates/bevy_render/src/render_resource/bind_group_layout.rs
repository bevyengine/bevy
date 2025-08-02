use crate::{define_atomic_id, renderer::RenderDevice, WgpuWrapper};
use bevy_ecs::system::Res;
use bevy_platform::sync::OnceLock;
use core::ops::Deref;

define_atomic_id!(BindGroupLayoutId);

/// Bind group layouts define the interface of resources (e.g. buffers, textures, samplers)
/// for a shader. The actual resource binding is done via a [`BindGroup`](super::BindGroup).
///
/// This is a lightweight thread-safe wrapper around wgpu's own [`BindGroupLayout`](wgpu::BindGroupLayout),
/// which can be cloned as needed to workaround lifetime management issues. It may be converted
/// from and dereferences to wgpu's [`BindGroupLayout`](wgpu::BindGroupLayout).
///
/// Can be created via [`RenderDevice::create_bind_group_layout`](crate::RenderDevice::create_bind_group_layout).
#[derive(Clone, Debug)]
pub struct BindGroupLayout {
    id: BindGroupLayoutId,
    value: WgpuWrapper<wgpu::BindGroupLayout>,
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
            value: WgpuWrapper::new(value),
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

static EMPTY_BIND_GROUP_LAYOUT: OnceLock<BindGroupLayout> = OnceLock::new();

pub(crate) fn init_empty_bind_group_layout(render_device: Res<RenderDevice>) {
    let layout = render_device.create_bind_group_layout(Some("empty_bind_group_layout"), &[]);
    EMPTY_BIND_GROUP_LAYOUT
        .set(layout)
        .expect("init_empty_bind_group_layout was called more than once");
}

pub fn empty_bind_group_layout() -> BindGroupLayout {
    EMPTY_BIND_GROUP_LAYOUT
        .get()
        .expect("init_empty_bind_group_layout was not called")
        .clone()
}

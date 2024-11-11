use crate::define_atomic_id;
use crate::renderer::WgpuWrapper;
use alloc::sync::Arc;
use core::ops::Deref;

define_atomic_id!(BindGroupLayoutId);

#[derive(Clone, Debug)]
pub struct BindGroupLayout {
    id: BindGroupLayoutId,
    value: Arc<WgpuWrapper<wgpu::BindGroupLayout>>,
}

impl PartialEq for BindGroupLayout {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl BindGroupLayout {
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
            value: Arc::new(WgpuWrapper::new(value)),
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

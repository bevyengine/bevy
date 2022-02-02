use bevy_reflect::Uuid;
use std::{ops::Deref, sync::Arc};

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct BindGroupLayoutId(Uuid);

#[derive(Clone, Debug)]
pub struct BindGroupLayout {
    id: BindGroupLayoutId,
    value: Arc<wgpu::BindGroupLayout>,
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
            id: BindGroupLayoutId(Uuid::new_v4()),
            value: Arc::new(value),
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

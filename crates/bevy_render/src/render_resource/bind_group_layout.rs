use crate::render_resource::resource_macros::*;
use bevy_reflect::Uuid;
use std::ops::Deref;

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct BindGroupLayoutId(Uuid);

render_resource_wrapper!(ErasedBindGroupLayout, wgpu::BindGroupLayout);

#[derive(Clone, Debug)]
pub struct BindGroupLayout {
    id: BindGroupLayoutId,
    value: ErasedBindGroupLayout,
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
            id: BindGroupLayoutId(Uuid::new_v4()),
            value: ErasedBindGroupLayout::new(value),
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

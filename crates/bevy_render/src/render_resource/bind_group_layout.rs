use crate::render_resource::resource_macros::*;
use bevy_reflect::Uuid;
use std::ops::Deref;

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct BindGroupLayoutId(Uuid);

#[derive(Clone, Debug)]
pub struct BindGroupLayout {
    id: BindGroupLayoutId,
    value: render_resource_type!(wgpu::BindGroupLayout),
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
        render_resource_ref!(&self.value, wgpu::BindGroupLayout)
    }
}

impl From<wgpu::BindGroupLayout> for BindGroupLayout {
    fn from(value: wgpu::BindGroupLayout) -> Self {
        BindGroupLayout {
            id: BindGroupLayoutId(Uuid::new_v4()),
            value: render_resource_new!(value),
        }
    }
}

impl Deref for BindGroupLayout {
    type Target = wgpu::BindGroupLayout;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.value()
    }
}

impl Drop for BindGroupLayout {
    fn drop(&mut self) {
        render_resource_drop!(&mut self.value, wgpu::BindGroupLayout);
    }
}

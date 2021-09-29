use bevy_reflect::Uuid;
use std::{ops::Deref, sync::Arc};

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct BindGroupId(Uuid);

#[derive(Clone, Debug)]
pub struct BindGroup {
    id: BindGroupId,
    value: Arc<wgpu::BindGroup>,
}

impl BindGroup {
    #[inline]
    pub fn id(&self) -> BindGroupId {
        self.id
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

impl Deref for BindGroup {
    type Target = wgpu::BindGroup;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

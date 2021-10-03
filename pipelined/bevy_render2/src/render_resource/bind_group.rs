use bevy_utils::{Id, IdType};
use std::sync::Arc;

#[derive(Id, Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct BindGroupId(IdType);

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

    #[inline]
    pub fn value(&self) -> &wgpu::BindGroup {
        &self.value
    }
}

impl From<wgpu::BindGroup> for BindGroup {
    fn from(value: wgpu::BindGroup) -> Self {
        BindGroup {
            id: BindGroupId::new(),
            value: Arc::new(value),
        }
    }
}

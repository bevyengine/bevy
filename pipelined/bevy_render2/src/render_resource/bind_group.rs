use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

static MAX_BIND_GROUP_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct BindGroupId(usize);

impl BindGroupId {
    /// Creates a new id by incrementing the atomic id counter.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(MAX_BIND_GROUP_ID.fetch_add(1, Ordering::Relaxed))
    }
}

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

use crate::render_resource::{next_id, Counter, Id};
use std::sync::Arc;

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct BindGroupId(Id);

impl BindGroupId {
    /// Creates a new, unique [`BindGroupId`].
    /// Returns [`None`] if the supply of unique ids has been exhausted.
    fn new() -> Option<Self> {
        static COUNTER: Counter = Counter::new(0);
        next_id(&COUNTER).map(Self)
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
            id: BindGroupId::new().expect("The system ran out of unique `BindGroupId`s."),
            value: Arc::new(value),
        }
    }
}

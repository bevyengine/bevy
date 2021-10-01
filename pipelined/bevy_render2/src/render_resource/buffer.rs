use crate::render_resource::{next_id, Counter, Id};
use std::{
    ops::{Bound, Deref, RangeBounds},
    sync::Arc,
};

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct BufferId(Id);

impl BufferId {
    /// Creates a new, unique [`BufferId`].
    /// Returns [`None`] if the supply of unique ids has been exhausted.
    fn new() -> Option<Self> {
        static COUNTER: Counter = Counter::new(0);
        next_id(&COUNTER).map(Self)
    }
}

#[derive(Clone, Debug)]
pub struct Buffer {
    id: BufferId,
    value: Arc<wgpu::Buffer>,
}

impl Buffer {
    #[inline]
    pub fn id(&self) -> BufferId {
        self.id
    }

    pub fn slice(&self, bounds: impl RangeBounds<wgpu::BufferAddress>) -> BufferSlice {
        BufferSlice {
            id: self.id,
            // need to compute and store this manually because wgpu doesn't export offset on wgpu::BufferSlice
            offset: match bounds.start_bound() {
                Bound::Included(&bound) => bound,
                Bound::Excluded(&bound) => bound + 1,
                Bound::Unbounded => 0,
            },
            value: self.value.slice(bounds),
        }
    }

    #[inline]
    pub fn unmap(&self) {
        self.value.unmap()
    }
}

impl From<wgpu::Buffer> for Buffer {
    fn from(value: wgpu::Buffer) -> Self {
        Buffer {
            id: BufferId::new().expect("The system ran out of unique `BufferId`s."),
            value: Arc::new(value),
        }
    }
}

impl Deref for Buffer {
    type Target = wgpu::Buffer;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

#[derive(Clone, Debug)]
pub struct BufferSlice<'a> {
    id: BufferId,
    offset: wgpu::BufferAddress,
    value: wgpu::BufferSlice<'a>,
}

impl<'a> BufferSlice<'a> {
    #[inline]
    pub fn id(&self) -> BufferId {
        self.id
    }

    #[inline]
    pub fn offset(&self) -> wgpu::BufferAddress {
        self.offset
    }
}

impl<'a> Deref for BufferSlice<'a> {
    type Target = wgpu::BufferSlice<'a>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

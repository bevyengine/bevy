use crate::define_atomic_id;
use crate::renderer::WgpuWrapper;
use core::ops::{Bound, Deref, RangeBounds};

define_atomic_id!(BufferId);

#[derive(Clone, Debug)]
pub struct Buffer {
    id: BufferId,
    value: WgpuWrapper<wgpu::Buffer>,
}

impl Buffer {
    #[inline]
    pub fn id(&self) -> BufferId {
        self.id
    }

    pub fn slice(&self, bounds: impl RangeBounds<wgpu::BufferAddress>) -> BufferSlice<'_> {
        // need to compute and store this manually because wgpu doesn't export offset and size on wgpu::BufferSlice
        let offset = match bounds.start_bound() {
            Bound::Included(&bound) => bound,
            Bound::Excluded(&bound) => bound + 1,
            Bound::Unbounded => 0,
        };
        let size = match bounds.end_bound() {
            Bound::Included(&bound) => bound + 1,
            Bound::Excluded(&bound) => bound,
            Bound::Unbounded => self.value.size(),
        } - offset;
        BufferSlice {
            id: self.id,
            offset,
            size,
            value: self.value.slice(bounds),
        }
    }

    #[inline]
    pub fn unmap(&self) {
        self.value.unmap();
    }
}

impl From<wgpu::Buffer> for Buffer {
    fn from(value: wgpu::Buffer) -> Self {
        Buffer {
            id: BufferId::new(),
            value: WgpuWrapper::new(value),
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
    size: wgpu::BufferAddress,
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

    #[inline]
    pub fn size(&self) -> wgpu::BufferAddress {
        self.size
    }
}

impl<'a> Deref for BufferSlice<'a> {
    type Target = wgpu::BufferSlice<'a>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

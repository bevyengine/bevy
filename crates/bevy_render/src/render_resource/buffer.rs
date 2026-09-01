use crate::renderer::{impl_eq_ord_hash_wrapper, wgpu_wrapper};
use core::ops::{Deref, RangeBounds};

wgpu_wrapper! {
    #[derive(Clone, Debug)]
    struct WgpuBuffer(wgpu::Buffer);
}

impl_eq_ord_hash_wrapper!(WgpuBuffer);

/// An opaque identifier for a [`Buffer`], backed by the wrapped wgpu [`Buffer`](wgpu::Buffer).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BufferId(WgpuBuffer);

#[derive(Clone, Debug)]
pub struct Buffer {
    value: WgpuBuffer,
}

impl Buffer {
    #[inline]
    pub fn id(&self) -> BufferId {
        BufferId(self.value.clone())
    }

    pub fn slice(&self, bounds: impl RangeBounds<wgpu::BufferAddress>) -> BufferSlice<'_> {
        BufferSlice {
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
            value: WgpuBuffer::new(value),
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
    value: wgpu::BufferSlice<'a>,
}

impl<'a> BufferSlice<'a> {
    #[inline]
    pub fn id(&self) -> BufferId {
        BufferId(WgpuBuffer::new(self.value.buffer().clone()))
    }
}

impl<'a> Deref for BufferSlice<'a> {
    type Target = wgpu::BufferSlice<'a>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

use crate::renderer::wgpu_wrapper;
use bevy_utils::define_atomic_id;
use core::ops::{Deref, RangeBounds};

define_atomic_id!(BufferId);

wgpu_wrapper! {
    #[derive(Clone, Debug)]
    struct WgpuBuffer(wgpu::Buffer);
}

#[derive(Clone, Debug)]
pub struct Buffer {
    id: BufferId,
    value: WgpuBuffer,
}

impl Buffer {
    #[inline]
    pub fn id(&self) -> BufferId {
        self.id
    }

    pub fn slice(&self, bounds: impl RangeBounds<wgpu::BufferAddress>) -> BufferSlice<'_> {
        BufferSlice {
            id: self.id,
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
    id: BufferId,
    value: wgpu::BufferSlice<'a>,
}

// SAFETY: `BufferSlice` is a `BufferId` plus a `wgpu::BufferSlice`, which holds a
// reference to a `wgpu::Buffer`. `BufferId` is an atomic ID and `wgpu::Buffer` is
// `Send + Sync` on every target where these impls are present; on wasm with atomics
// it is not, so the impls are omitted.
#[cfg(not(all(target_arch = "wasm32", target_feature = "atomics")))]
unsafe impl Send for BufferSlice<'_> {}
// SAFETY: `BufferSlice` is a `BufferId` plus a `wgpu::BufferSlice`, which holds a
// reference to a `wgpu::Buffer`. `BufferId` is an atomic ID and `wgpu::Buffer` is
// `Send + Sync` on every target where these impls are present; on wasm with atomics
// it is not, so the impls are omitted.
#[cfg(not(all(target_arch = "wasm32", target_feature = "atomics")))]
unsafe impl Sync for BufferSlice<'_> {}

impl<'a> BufferSlice<'a> {
    #[inline]
    pub fn id(&self) -> BufferId {
        self.id
    }
}

impl<'a> Deref for BufferSlice<'a> {
    type Target = wgpu::BufferSlice<'a>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

use crate::frame_graph::{
    BindGroupResourceHandle, BindGroupResourceHandleHelper, BindingResourceBufferHandle,
    BufferInfo, FrameGraph, TransientBuffer, Handle, IntoBindGroupResourceHandle,
};
use crate::renderer::WgpuWrapper;
use crate::{define_atomic_id, frame_graph::ResourceMaterial};
use core::ops::{Bound, Deref, RangeBounds};
use std::sync::Arc;

define_atomic_id!(BufferId);

#[derive(Clone, Debug)]
pub struct Buffer {
    id: BufferId,
    value: WgpuWrapper<wgpu::Buffer>,
    desc: BufferInfo,
}

impl BindGroupResourceHandleHelper for Buffer {
    fn make_bind_group_resource_handle(
        &self,
        frame_graph: &mut FrameGraph,
    ) -> BindGroupResourceHandle {
        let buffer = self.imported(frame_graph);

        BindingResourceBufferHandle { buffer, size: None, offset: 0 }.into_binding()
    }
}

impl ResourceMaterial for Buffer {
    type ResourceType = TransientBuffer;

    fn imported(&self, frame_graph: &mut FrameGraph) -> Handle<TransientBuffer> {
        let key = format!("buffer_{:?}", self.id());
        let buffer = Arc::new(TransientBuffer {
            resource: self.value.deref().clone(),
            desc: self.desc.clone(),
        });
        let handle = frame_graph.import(&key, buffer);
        handle
    }
}

impl Buffer {
    pub fn new(value: wgpu::Buffer, desc: BufferInfo) -> Self {
        Self {
            id: BufferId::new(),
            value: WgpuWrapper::new(value),
            desc,
        }
    }

    #[inline]
    pub fn id(&self) -> BufferId {
        self.id
    }

    pub fn slice(&self, bounds: impl RangeBounds<wgpu::BufferAddress>) -> BufferSlice {
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

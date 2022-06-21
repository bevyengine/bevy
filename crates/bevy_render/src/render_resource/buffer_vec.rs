use crate::{
    render_resource::Buffer,
    renderer::{RenderDevice, RenderQueue},
};
use bevy_core::{cast_slice, Pod};
use copyless::VecHelper;
use wgpu::BufferUsages;

/// A structure for storing raw bytes that have already been properly formatted
/// for use by the GPU.
///
/// "Properly formatted" means data that is [`#[repr(C)]`](https://doc.rust-lang.org/reference/type-layout.html#the-c-representation), [`Pod`]
/// and `Zeroable`. Unlike the [`DynamicUniformVec`](crate::render_resource::DynamicUniformVec),
/// [`UniformVec`](crate::render_resource::UniformVec) and [`StorageBuffer`](crate::render_resource::StorageBuffer) structures,
/// `BufferVec` does not provide padding or alignment between items, or within items.
///
/// The data is stored on the host, calling [`reserve`](crate::render_resource::BufferVec::reserve)
/// allocates memory on the [`RenderDevice`](crate::renderer::RenderDevice).
/// [`write_buffer`](crate::render_resource::BufferVec::write_buffer) queues copying of the data
/// from the host to the [`RenderDevice`](crate::renderer::RenderDevice).
pub struct BufferVec<T: Pod> {
    values: Vec<T>,
    buffer: Option<Buffer>,
    capacity: usize,
    item_size: usize,
    buffer_usage: BufferUsages,
}

impl<T: Pod> Default for BufferVec<T> {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            buffer: None,
            capacity: 0,
            buffer_usage: BufferUsages::all(),
            item_size: std::mem::size_of::<T>(),
        }
    }
}

impl<T: Pod> BufferVec<T> {
    pub fn new(buffer_usage: BufferUsages) -> Self {
        Self {
            buffer_usage,
            ..Default::default()
        }
    }

    #[inline]
    pub fn buffer(&self) -> Option<&Buffer> {
        self.buffer.as_ref()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn push(&mut self, value: T) -> usize {
        let index = self.values.len();
        self.values.alloc().init(value);
        index
    }

    /// Creates a [`Buffer`](crate::render_resource::Buffer) on the [`RenderDevice`](crate::renderer::RenderDevice) with size
    /// at least `std::mem::size_of::<T>() * capacity`, unless a such a buffer already exists.
    ///
    /// If a [`Buffer`](crate::render_resource::Buffer) exists, but is too small, references to it will be discarded,
    /// and a new [`Buffer`](crate::render_resource::Buffer) will be created. Any previously created [`Buffer`](crate::render_resource::Buffer)s
    /// that are no longer referenced will be deleted by the [`RenderDevice`](crate::renderer::RenderDevice)
    /// once it is done using them (typically 1-2 frames).
    ///
    /// In addition to any [`BufferUsages`](crate::render_resource::BufferUsages) provided when
    /// the `BufferVec` was created, the buffer on the [`RenderDevice`](crate::renderer::RenderDevice)
    /// is marked as [`BufferUsages::COPY_DST`](crate::render_resource::BufferUsages).
    pub fn reserve(&mut self, capacity: usize, device: &RenderDevice) {
        if capacity > self.capacity {
            self.capacity = capacity;
            let size = self.item_size * capacity;
            self.buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: size as wgpu::BufferAddress,
                usage: BufferUsages::COPY_DST | self.buffer_usage,
                mapped_at_creation: false,
            }));
        }
    }

    /// Queues writing of memory from the host to [`RenderDevice`](crate::renderer::RenderDevice) using
    /// the provided [`RenderQueue`](crate::renderer::RenderQueue).
    ///
    /// Before queueing the write, a [`reserve`](crate::render_resource::BufferVec::reserve) operation
    /// is executed.
    pub fn write_buffer(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        if self.values.is_empty() {
            return;
        }
        self.reserve(self.values.len(), device);
        if let Some(buffer) = &self.buffer {
            let range = 0..self.item_size * self.values.len();
            let bytes: &[u8] = cast_slice(&self.values);
            queue.write_buffer(buffer, 0, &bytes[range]);
        }
    }

    pub fn clear(&mut self) {
        self.values.clear();
    }
}

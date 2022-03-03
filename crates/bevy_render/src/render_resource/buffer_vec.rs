use crate::{
    render_resource::Buffer,
    renderer::{RenderDevice, RenderQueue},
};
use bevy_core::{cast_slice, Pod};
use std::ops::{Deref, DerefMut};
use wgpu::BufferUsages;

/// A user-friendly wrapper around a [`Buffer`] that provides a `Vec`-like
/// interface for constructing the buffer.
pub struct BufferVec<T: Pod> {
    values: Vec<T>,
    buffer: Option<Buffer>,
    capacity: usize,
    buffer_usage: BufferUsages,
}

impl<T: Pod> Default for BufferVec<T> {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            buffer: None,
            capacity: 0,
            buffer_usage: BufferUsages::all(),
        }
    }
}

impl<T: Pod> BufferVec<T> {
    /// Creates a new [`BufferVec`] with the associated [`BufferUsages`].
    ///
    /// This does not immediately allocate a system/video RAM buffers.
    pub fn new(buffer_usage: BufferUsages) -> Self {
        Self {
            buffer_usage,
            ..Default::default()
        }
    }

    /// Gets the reference to the underlying buffer, if one has been allocated.
    #[inline]
    pub fn buffer(&self) -> Option<&Buffer> {
        self.buffer.as_ref()
    }

    /// Queues up a copy of the contents of the [`BufferVec`] into the underlying
    /// buffer.
    ///
    /// If no buffer has been allocated yet or if the current size of the contents
    /// exceeds the size of the underlying buffer, a new buffer will be allocated.
    pub fn write_buffer(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        if self.values.is_empty() {
            return;
        }
        self.reserve_buffer(self.values.len(), device);
        if let Some(buffer) = &self.buffer {
            let range = 0..self.size();
            let bytes: &[u8] = cast_slice(&self.values);
            queue.write_buffer(buffer, 0, &bytes[range]);
        }
    }

    /// Consumes the [`BufferVec`] and returns the underlying [`Vec`].
    /// If a buffer was allocated, it will be dropped.
    pub fn take_vec(self) -> Vec<T> {
        self.values
    }

    /// Consumes the [`BufferVec`] and returns the underlying [`Buffer`]
    /// if one was allocated.
    pub fn take_buffer(self) -> Option<Buffer> {
        self.buffer
    }

    fn reserve_buffer(&mut self, capacity: usize, device: &RenderDevice) {
        if capacity > self.capacity {
            self.capacity = capacity;
            self.buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: self.size() as wgpu::BufferAddress,
                usage: BufferUsages::COPY_DST | self.buffer_usage,
                mapped_at_creation: false,
            }));
        }
    }

    fn size(&self) -> usize {
        std::mem::size_of::<T>() * self.values.len()
    }
}

impl<T: Pod> Deref for BufferVec<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

impl<T: Pod> DerefMut for BufferVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.values
    }
}

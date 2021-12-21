use crate::{
    render_resource::Buffer,
    renderer::{RenderDevice, RenderQueue},
};
use bevy_core::{cast_slice, Pod};
use wgpu::BufferUsages;

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
        self.values.push(value);
        index
    }

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

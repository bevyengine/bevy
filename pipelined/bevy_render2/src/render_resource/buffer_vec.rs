use crate::{
    render_resource::Buffer,
    renderer::{RenderDevice, RenderQueue},
};
use bevy_core::{cast_slice, Pod};
use wgpu::BufferUsage;

pub struct BufferVec<T: Pod> {
    values: Vec<T>,
    buffer: Option<Buffer>,
    capacity: usize,
    item_size: usize,
    buffer_usage: BufferUsage,
}

impl<T: Pod> Default for BufferVec<T> {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            buffer: None,
            capacity: 0,
            buffer_usage: BufferUsage::all(),
            item_size: std::mem::size_of::<T>(),
        }
    }
}

impl<T: Pod> BufferVec<T> {
    pub fn new(buffer_usage: BufferUsage) -> Self {
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

    pub fn push(&mut self, value: T) -> usize {
        let len = self.values.len();
        if len < self.capacity {
            self.values.push(value);
            len
        } else {
            panic!(
                "Cannot push value because capacity of {} has been reached",
                self.capacity
            );
        }
    }

    pub fn reserve(&mut self, capacity: usize, device: &RenderDevice) {
        if capacity > self.capacity {
            self.capacity = capacity;
            let size = self.item_size * capacity;
            self.buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: size as wgpu::BufferAddress,
                usage: BufferUsage::COPY_DST | self.buffer_usage,
                mapped_at_creation: false,
            }));
        }
    }

    pub fn reserve_and_clear(&mut self, capacity: usize, device: &RenderDevice) {
        self.clear();
        self.reserve(capacity, device);
    }

    pub fn write_buffer(&mut self, queue: &RenderQueue) {
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

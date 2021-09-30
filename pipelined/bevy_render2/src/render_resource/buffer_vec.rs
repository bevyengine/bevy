use crate::{render_resource::Buffer, renderer::GpuDevice};
use bevy_core::{cast_slice, Pod};
use wgpu::BufferUsage;

pub struct BufferVec<T: Pod> {
    values: Vec<T>,
    staging_buffer: Option<Buffer>,
    buffer: Option<Buffer>,
    capacity: usize,
    item_size: usize,
    buffer_usage: BufferUsage,
}

impl<T: Pod> Default for BufferVec<T> {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            staging_buffer: None,
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
    pub fn staging_buffer(&self) -> Option<&Buffer> {
        self.staging_buffer.as_ref()
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
        if self.values.len() < self.capacity {
            let index = self.values.len();
            self.values.push(value);
            index
        } else {
            panic!(
                "Cannot push value because capacity of {} has been reached",
                self.capacity
            );
        }
    }

    pub fn reserve(&mut self, capacity: usize, gpu_device: &GpuDevice) {
        if capacity > self.capacity {
            self.capacity = capacity;
            let size = (self.item_size * capacity) as wgpu::BufferAddress;
            self.staging_buffer = Some(gpu_device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size,
                usage: BufferUsage::COPY_SRC | BufferUsage::MAP_WRITE,
                mapped_at_creation: false,
            }));
            self.buffer = Some(gpu_device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size,
                usage: BufferUsage::COPY_DST | self.buffer_usage,
                mapped_at_creation: false,
            }));
        }
    }

    pub fn reserve_and_clear(&mut self, capacity: usize, gpu_device: &GpuDevice) {
        self.clear();
        self.reserve(capacity, gpu_device);
    }

    pub fn write_to_staging_buffer(&self, gpu_device: &GpuDevice) {
        if let Some(staging_buffer) = &self.staging_buffer {
            let end = (self.values.len() * self.item_size) as u64;
            let slice = staging_buffer.slice(0..end);
            gpu_device.map_buffer(&slice, wgpu::MapMode::Write);
            {
                let mut data = slice.get_mapped_range_mut();
                let bytes: &[u8] = cast_slice(&self.values);
                data.copy_from_slice(bytes);
            }
            staging_buffer.unmap();
        }
    }
    pub fn write_to_buffer(&self, command_encoder: &mut wgpu::CommandEncoder) {
        if let (Some(staging_buffer), Some(uniform_buffer)) = (&self.staging_buffer, &self.buffer) {
            command_encoder.copy_buffer_to_buffer(
                staging_buffer,
                0,
                uniform_buffer,
                0,
                (self.values.len() * self.item_size) as u64,
            );
        }
    }

    pub fn clear(&mut self) {
        self.values.clear();
    }
}

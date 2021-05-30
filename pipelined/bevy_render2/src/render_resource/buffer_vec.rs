use crate::{
    render_resource::{BufferId, BufferInfo, BufferMapMode, BufferUsage},
    renderer::{RenderContext, RenderResources},
};
use bevy_core::{cast_slice, Pod};

pub struct BufferVec<T: Pod> {
    values: Vec<T>,
    staging_buffer: Option<BufferId>,
    buffer: Option<BufferId>,
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
    pub fn staging_buffer(&self) -> Option<BufferId> {
        self.staging_buffer
    }

    #[inline]
    pub fn buffer(&self) -> Option<BufferId> {
        self.buffer
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

    pub fn reserve(&mut self, capacity: usize, render_resources: &RenderResources) {
        if capacity > self.capacity {
            self.capacity = capacity;
            if let Some(staging_buffer) = self.staging_buffer.take() {
                render_resources.remove_buffer(staging_buffer);
            }

            if let Some(buffer) = self.buffer.take() {
                render_resources.remove_buffer(buffer);
            }

            let size = self.item_size * capacity;
            self.staging_buffer = Some(render_resources.create_buffer(BufferInfo {
                size,
                buffer_usage: BufferUsage::COPY_SRC | BufferUsage::MAP_WRITE,
                mapped_at_creation: false,
            }));
            self.buffer = Some(render_resources.create_buffer(BufferInfo {
                size,
                buffer_usage: BufferUsage::COPY_DST | self.buffer_usage,
                mapped_at_creation: false,
            }));
        }
    }

    pub fn reserve_and_clear(&mut self, capacity: usize, render_resources: &RenderResources) {
        self.clear();
        self.reserve(capacity, render_resources);
    }

    pub fn write_to_staging_buffer(&self, render_resources: &RenderResources) {
        if let Some(staging_buffer) = self.staging_buffer {
            let size = self.values.len() * self.item_size;
            render_resources.map_buffer(staging_buffer, BufferMapMode::Write);
            render_resources.write_mapped_buffer(
                staging_buffer,
                0..size as u64,
                &mut |data, _renderer| {
                    let bytes: &[u8] = cast_slice(&self.values);
                    data.copy_from_slice(bytes);
                },
            );
            render_resources.unmap_buffer(staging_buffer);
        }
    }
    pub fn write_to_uniform_buffer(&self, render_context: &mut dyn RenderContext) {
        if let (Some(staging_buffer), Some(uniform_buffer)) = (self.staging_buffer, self.buffer) {
            render_context.copy_buffer_to_buffer(
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

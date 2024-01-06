use bevy_render::{
    render_resource::{
        BindingResource, Buffer, BufferDescriptor, BufferUsages, CommandEncoderDescriptor,
    },
    renderer::{RenderDevice, RenderQueue},
};
use std::ops::Range;

/// Wrapper for a GPU buffer holding a large amount of persistent data.
pub struct PersistentGpuBuffer<T: PersistentGpuBufferable> {
    /// Debug label for the buffer.
    label: &'static str,
    /// Handle to the GPU buffer.
    buffer: Buffer,
    /// Queue of pending writes, and associated metadata.
    write_queue: Vec<(T, T::Metadata)>,
    /// Queue of pending writes in byte form.
    upload_buffer: Vec<u8>,
    /// The next offset into the buffer to be used when queueing new data to be written.
    next_queued_write_address: u64,
    /// The offset into the buffer to be used for writing bytes.
    next_write_address: u64,
}

impl<T: PersistentGpuBufferable> PersistentGpuBuffer<T> {
    /// Create a new persistent buffer.
    pub fn new(label: &'static str, render_device: &RenderDevice) -> Self {
        Self {
            label,
            buffer: render_device.create_buffer(&BufferDescriptor {
                label: Some(label),
                size: 0,
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }),
            write_queue: Vec::with_capacity(50),
            upload_buffer: Vec::new(),
            next_queued_write_address: 0,
            next_write_address: 0,
        }
    }

    /// Queue an item of type T to be added to the buffer, returning the byte range within the buffer that it will be located at.
    pub fn queue_write(&mut self, data: T, metadata: T::Metadata) -> Range<u64> {
        let start_address = self.next_queued_write_address;
        self.next_queued_write_address += data.size_in_bytes();

        self.write_queue.push((data, metadata));

        start_address..self.next_queued_write_address
    }

    /// Upload all pending data to the GPU buffer.
    pub fn perform_writes(&mut self, render_queue: &RenderQueue, render_device: &RenderDevice) {
        // If the queued data would overflow the buffer, expand it.
        if self.next_queued_write_address >= self.buffer.size() {
            self.expand_buffer(render_device, render_queue);
        }

        let queue_count = self.write_queue.len();

        // Serialize all items into the upload buffer
        self.upload_buffer.clear();
        for (data, metadata) in self.write_queue.drain(..) {
            data.write_bytes_le(metadata, &mut self.upload_buffer);
        }

        // Upload the upload buffer to the GPU
        render_queue.write_buffer(&self.buffer, self.next_write_address, &self.upload_buffer);
        self.next_write_address = self.next_queued_write_address;

        let queue_saturation = queue_count as f32 / self.write_queue.capacity() as f32;
        if queue_saturation < 0.3 {
            self.write_queue.shrink_to(50);
        }

        let upload_saturation =
            self.upload_buffer.len() as f32 / self.upload_buffer.capacity() as f32;
        if upload_saturation < 0.1 {
            self.write_queue = Vec::new();
        }
    }

    pub fn binding(&self) -> BindingResource<'_> {
        self.buffer.as_entire_binding()
    }

    // Expand the buffer by creating a new buffer and copying old data over.
    fn expand_buffer(&mut self, render_device: &RenderDevice, render_queue: &RenderQueue) {
        let size = (self.buffer.size() * 2).max(4 + self.next_queued_write_address);

        let new_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some(self.label),
            size,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let mut command_encoder = render_device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("persistent_gpu_buffer_expand"),
        });
        command_encoder.copy_buffer_to_buffer(&self.buffer, 0, &new_buffer, 0, self.buffer.size());
        render_queue.submit([command_encoder.finish()]);

        self.buffer = new_buffer;
    }
}

/// A trait representing data that can be written to a [`PersistentGpuBuffer`].
///
/// SAFETY: All data must be a multiple of `wgpu::COPY_BUFFER_ALIGNMENT` bytes.
/// SAFETY: The amount of bytes written to `buffer` in `write_bytes_le()` must match `size_in_bytes()`.
pub trait PersistentGpuBufferable {
    /// Additional metadata associated with each item, made available during [`write_bytes_le`].
    type Metadata;

    /// The size in bytes of `self`.
    fn size_in_bytes(&self) -> u64;

    /// Convert `self` + `metadata` into bytes (little-endian), and write to the provided buffer.
    fn write_bytes_le(&self, metadata: Self::Metadata, buffer: &mut Vec<u8>);
}

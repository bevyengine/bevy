use bevy_render::{
    render_resource::{
        BindingResource, Buffer, BufferDescriptor, BufferUsages, CommandEncoderDescriptor,
    },
    renderer::{RenderDevice, RenderQueue},
};
use std::ops::Range;

pub struct PersistentGpuBuffer<T: PersistentGpuBufferable> {
    label: &'static str,
    buffer: Buffer,
    write_queue: Vec<T>,
    last_write_address: u64,
    next_queued_write_address: u64,
}

impl<T: PersistentGpuBufferable> PersistentGpuBuffer<T> {
    pub fn new(label: &'static str, render_device: &RenderDevice) -> Self {
        Self {
            label,
            buffer: render_device.create_buffer(&BufferDescriptor {
                label: Some(label),
                size: 0,
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }),
            write_queue: Vec::new(),
            last_write_address: 0,
            next_queued_write_address: 0,
        }
    }

    pub fn queue_write(&mut self, data: T) -> Range<u64> {
        let start_address = self.next_queued_write_address;
        self.next_queued_write_address += data.size_in_bytes();

        self.write_queue.push(data);

        start_address..self.next_queued_write_address
    }

    pub fn perform_writes(&mut self, render_queue: &RenderQueue, render_device: &RenderDevice) {
        if self.next_queued_write_address >= self.buffer.size() {
            self.expand_buffer(render_device, render_queue);
        }

        // TODO: Maybe create a large storage buffer to use as the staging buffer,
        // instead of many small writes?
        for item in self.write_queue.drain(..) {
            let bytes = item.as_bytes_le(self.last_write_address);
            render_queue.write_buffer(&self.buffer, self.last_write_address, bytes);
            self.last_write_address += bytes.len() as u64;
        }
    }

    pub fn binding(&self) -> BindingResource<'_> {
        self.buffer.as_entire_binding()
    }

    fn expand_buffer(&mut self, render_device: &RenderDevice, render_queue: &RenderQueue) {
        let new_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some(self.label),
            size: self.buffer.size() * 2,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let mut command_encoder = render_device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("persistent_storage_buffer_expand"),
        });
        command_encoder.copy_buffer_to_buffer(&self.buffer, 0, &new_buffer, 0, self.buffer.size());
        render_queue.submit([command_encoder.finish()]);

        self.buffer = new_buffer;
    }
}

/// SAFETY: All data must be a multiple of wgpu::COPY_BUFFER_ALIGNMENT bytes.
/// The size given by size_in_bytes() must match as_bytes_le().
pub trait PersistentGpuBufferable {
    fn size_in_bytes(&self) -> u64;

    fn as_bytes_le(&self, start_address: u64) -> &[u8];
}

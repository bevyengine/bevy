use bevy_render::{
    render_resource::{
        BindingResource, Buffer, BufferDescriptor, BufferUsages, CommandEncoderDescriptor,
    },
    renderer::{RenderDevice, RenderQueue},
};
use std::{borrow::Cow, ops::Range};

pub struct PersistentGpuBuffer<T: PersistentGpuBufferable> {
    label: &'static str,
    buffer: Buffer,
    write_queue: Vec<(T, T::ExtraData)>,
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
            write_queue: Vec::with_capacity(50),
            last_write_address: 0,
            next_queued_write_address: 0,
        }
    }

    pub fn queue_write(&mut self, data: T, extra_data: T::ExtraData) -> Range<u64> {
        let start_address = self.next_queued_write_address;
        self.next_queued_write_address += data.size_in_bytes();

        self.write_queue.push((data, extra_data));

        start_address..self.next_queued_write_address
    }

    pub fn perform_writes(&mut self, render_queue: &RenderQueue, render_device: &RenderDevice) {
        if self.next_queued_write_address >= self.buffer.size() {
            self.expand_buffer(render_device, render_queue);
        }

        let queue_count = self.write_queue.len();

        // TODO: Create a large storage buffer to use as the staging buffer,
        // and write bytes directly into it, instead of many small GPU writes.
        //
        // Have PersistentGpuBufferable take a &mut Vec<u8> to write to.
        for (data, extra_data) in self.write_queue.drain(..) {
            let bytes = data.as_bytes_le(extra_data);
            render_queue.write_buffer(&self.buffer, self.last_write_address, &bytes);
            self.last_write_address += bytes.len() as u64;
        }

        let queue_saturation = queue_count as f32 / self.write_queue.capacity() as f32;
        if queue_saturation < 0.3 {
            self.write_queue.shrink_to(50);
        }
    }

    pub fn binding(&self) -> BindingResource<'_> {
        self.buffer.as_entire_binding()
    }

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

/// SAFETY: All data must be a multiple of wgpu::COPY_BUFFER_ALIGNMENT bytes.
/// The size given by size_in_bytes() must match as_bytes_le().
pub trait PersistentGpuBufferable {
    type ExtraData;

    fn size_in_bytes(&self) -> u64;

    fn as_bytes_le<'a>(&'a self, extra_data: Self::ExtraData) -> Cow<'a, [u8]>;
}

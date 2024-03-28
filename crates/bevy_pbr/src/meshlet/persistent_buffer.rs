use bevy_render::{
    render_resource::{
        BindingResource, Buffer, BufferAddress, BufferDescriptor, BufferUsages,
        CommandEncoderDescriptor, COPY_BUFFER_ALIGNMENT,
    },
    renderer::{RenderDevice, RenderQueue},
};
use range_alloc::RangeAllocator;
use std::{num::NonZeroU64, ops::Range};

/// Wrapper for a GPU buffer holding a large amount of data that persists across frames.
pub struct PersistentGpuBuffer<T: PersistentGpuBufferable> {
    /// Debug label for the buffer.
    label: &'static str,
    /// Handle to the GPU buffer.
    buffer: Buffer,
    /// Tracks free slices of the buffer.
    allocation_planner: RangeAllocator<BufferAddress>,
    /// Queue of pending writes, and associated metadata.
    write_queue: Vec<(T, T::Metadata, Range<BufferAddress>)>,
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
            allocation_planner: RangeAllocator::new(0..0),
            write_queue: Vec::new(),
        }
    }

    /// Queue an item of type T to be added to the buffer, returning the byte range within the buffer that it will be located at.
    pub fn queue_write(&mut self, data: T, metadata: T::Metadata) -> Range<BufferAddress> {
        let data_size = data.size_in_bytes() as u64;
        debug_assert!(data_size % COPY_BUFFER_ALIGNMENT == 0);
        if let Ok(buffer_slice) = self.allocation_planner.allocate_range(data_size) {
            self.write_queue
                .push((data, metadata, buffer_slice.clone()));
            return buffer_slice;
        }

        let buffer_size = self.allocation_planner.initial_range();
        let double_buffer_size = (buffer_size.end - buffer_size.start) * 2;
        let new_size = double_buffer_size.max(data_size);
        self.allocation_planner.grow_to(buffer_size.end + new_size);

        let buffer_slice = self.allocation_planner.allocate_range(data_size).unwrap();
        self.write_queue
            .push((data, metadata, buffer_slice.clone()));
        buffer_slice
    }

    /// Upload all pending data to the GPU buffer.
    pub fn perform_writes(&mut self, render_queue: &RenderQueue, render_device: &RenderDevice) {
        if self.allocation_planner.initial_range().end > self.buffer.size() {
            self.expand_buffer(render_device, render_queue);
        }

        let queue_count = self.write_queue.len();

        for (data, metadata, buffer_slice) in self.write_queue.drain(..) {
            let buffer_slice_size = NonZeroU64::new(buffer_slice.end - buffer_slice.start).unwrap();
            let mut buffer_view = render_queue
                .write_buffer_with(&self.buffer, buffer_slice.start, buffer_slice_size)
                .unwrap();
            data.write_bytes_le(metadata, &mut buffer_view);
        }

        let queue_saturation = queue_count as f32 / self.write_queue.capacity() as f32;
        if queue_saturation < 0.3 {
            self.write_queue = Vec::new();
        }
    }

    /// Mark a section of the GPU buffer as no longer needed.
    pub fn mark_slice_unused(&mut self, buffer_slice: Range<BufferAddress>) {
        self.allocation_planner.free_range(buffer_slice);
    }

    pub fn binding(&self) -> BindingResource<'_> {
        self.buffer.as_entire_binding()
    }

    /// Expand the buffer by creating a new buffer and copying old data over.
    fn expand_buffer(&mut self, render_device: &RenderDevice, render_queue: &RenderQueue) {
        let size = self.allocation_planner.initial_range();
        let new_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some(self.label),
            size: size.end - size.start,
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
pub trait PersistentGpuBufferable {
    /// Additional metadata associated with each item, made available during `write_bytes_le`.
    type Metadata;

    /// The size in bytes of `self`. This will determine the size of the buffer passed into
    /// `write_bytes_le`.
    ///
    /// All data written must be in a multiple of `wgpu::COPY_BUFFER_ALIGNMENT` bytes. Failure to do so will
    /// result in a panic when using [`PersistentGpuBuffer`].
    fn size_in_bytes(&self) -> usize;

    /// Convert `self` + `metadata` into bytes (little-endian), and write to the provided buffer slice.
    /// Any bytes not written to in the slice will be zeroed out when uploaded to the GPU.
    fn write_bytes_le(&self, metadata: Self::Metadata, buffer_slice: &mut [u8]);
}

use super::{BufferId, BufferInfo, RenderResource, RenderResourceBinding};
use crate::{
    render_graph::CommandQueue,
    renderer::{BufferUsage, RenderContext, RenderResourceContext},
};
use bevy_ecs::{Res, ResMut};

pub struct SharedBuffers {
    staging_buffer: Option<BufferId>,
    uniform_buffer: Option<BufferId>,
    buffers_to_free: Vec<BufferId>,
    buffer_size: usize,
    initial_size: usize,
    current_offset: usize,
    command_queue: CommandQueue,
}

impl SharedBuffers {
    pub fn new(initial_size: usize) -> Self {
        Self {
            staging_buffer: None,
            uniform_buffer: None,
            buffer_size: 0,
            current_offset: 0,
            initial_size,
            buffers_to_free: Default::default(),
            command_queue: Default::default(),
        }
    }

    pub fn grow(
        &mut self,
        render_resource_context: &dyn RenderResourceContext,
        required_space: usize,
    ) {
        while self.buffer_size < self.current_offset + required_space {
            self.buffer_size = if self.buffer_size == 0 {
                self.initial_size
            } else {
                self.buffer_size * 2
            };
        }

        self.current_offset = 0;

        if let Some(staging_buffer) = self.staging_buffer.take() {
            render_resource_context.unmap_buffer(staging_buffer);
            self.buffers_to_free.push(staging_buffer);
        }

        if let Some(uniform_buffer) = self.uniform_buffer.take() {
            self.buffers_to_free.push(uniform_buffer);
        }

        self.staging_buffer = Some(render_resource_context.create_buffer(BufferInfo {
            size: self.buffer_size,
            buffer_usage: BufferUsage::MAP_WRITE | BufferUsage::COPY_SRC,
            mapped_at_creation: true,
        }));
        self.uniform_buffer = Some(render_resource_context.create_buffer(BufferInfo {
            size: self.buffer_size,
            buffer_usage: BufferUsage::COPY_DST | BufferUsage::UNIFORM,
            mapped_at_creation: false,
        }));
    }

    pub fn get_uniform_buffer<T: RenderResource>(
        &mut self,
        render_resource_context: &dyn RenderResourceContext,
        render_resource: &T,
    ) -> Option<RenderResourceBinding> {
        if let Some(size) = render_resource.buffer_byte_len() {
            // TODO: overlap alignment if/when possible
            let aligned_size = render_resource_context.get_aligned_uniform_size(size, true);
            let mut new_offset = self.current_offset + aligned_size;
            if new_offset > self.buffer_size {
                self.grow(render_resource_context, aligned_size);
                new_offset = aligned_size;
            }

            let range = self.current_offset as u64..new_offset as u64;
            let staging_buffer = self.staging_buffer.unwrap();
            let uniform_buffer = self.uniform_buffer.unwrap();
            render_resource_context.write_mapped_buffer(
                staging_buffer,
                range.clone(),
                &mut |data, _renderer| {
                    render_resource.write_buffer_bytes(data);
                },
            );

            self.command_queue.copy_buffer_to_buffer(
                staging_buffer,
                self.current_offset as u64,
                uniform_buffer,
                self.current_offset as u64,
                aligned_size as u64,
            );

            self.current_offset = new_offset;
            Some(RenderResourceBinding::Buffer {
                buffer: uniform_buffer,
                range,
                dynamic_index: None,
            })
        } else {
            None
        }
    }

    pub fn update(&mut self, render_resource_context: &dyn RenderResourceContext) {
        self.current_offset = 0;
        for buffer in self.buffers_to_free.drain(..) {
            render_resource_context.remove_buffer(buffer)
        }

        if let Some(staging_buffer) = self.staging_buffer {
            render_resource_context.map_buffer(staging_buffer);
        }
    }

    pub fn apply(&mut self, render_context: &mut dyn RenderContext) {
        if let Some(staging_buffer) = self.staging_buffer {
            render_context.resources().unmap_buffer(staging_buffer);
        }
        let mut command_queue = std::mem::take(&mut self.command_queue);
        command_queue.execute(render_context);
    }
}

pub fn shared_buffers_update_system(
    mut shared_buffers: ResMut<SharedBuffers>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
) {
    shared_buffers.update(&**render_resource_context);
}

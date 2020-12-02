use super::{BufferId, BufferInfo, RenderResource, RenderResourceBinding};
use crate::{
    render_graph::CommandQueue,
    renderer::{BufferUsage, RenderContext, RenderResourceContext},
};
use bevy_ecs::{Res, ResMut};

pub struct SharedBuffers {
    staging_buffer: BufferId,
    uniform_buffer: BufferId,
    buffers_to_free: Vec<BufferId>,
    buffer_size: usize,
    initial_size: usize,
    current_offset: usize,
    command_queue: CommandQueue,
}

impl SharedBuffers {
    pub fn new(initial_size: usize) -> Self {
        Self {
            staging_buffer: BufferId::new(), // non-existent buffer
            uniform_buffer: BufferId::new(), // non-existent buffer
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
        let first_resize = self.buffer_size == 0;
        while self.buffer_size < self.current_offset + required_space {
            self.buffer_size = if self.buffer_size == 0 {
                self.initial_size
            } else {
                self.buffer_size * 2
            };
        }

        self.current_offset = 0;

        // ignore the initial "dummy buffers"
        if !first_resize {
            render_resource_context.unmap_buffer(self.staging_buffer);
            self.buffers_to_free.push(self.staging_buffer);
            self.buffers_to_free.push(self.uniform_buffer);
        }

        self.staging_buffer = render_resource_context.create_buffer(BufferInfo {
            size: self.buffer_size,
            buffer_usage: BufferUsage::MAP_WRITE | BufferUsage::COPY_SRC,
            mapped_at_creation: true,
        });
        self.uniform_buffer = render_resource_context.create_buffer(BufferInfo {
            size: self.buffer_size,
            buffer_usage: BufferUsage::COPY_DST | BufferUsage::UNIFORM,
            mapped_at_creation: false,
        });
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

            render_resource_context.write_mapped_buffer(
                self.staging_buffer,
                range.clone(),
                &mut |data, _renderer| {
                    render_resource.write_buffer_bytes(data);
                },
            );

            self.command_queue.copy_buffer_to_buffer(
                self.staging_buffer,
                self.current_offset as u64,
                self.uniform_buffer,
                self.current_offset as u64,
                aligned_size as u64,
            );

            self.current_offset = new_offset;
            Some(RenderResourceBinding::Buffer {
                buffer: self.uniform_buffer,
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
        render_resource_context.map_buffer(self.staging_buffer);
    }

    pub fn apply(&mut self, render_context: &mut dyn RenderContext) {
        render_context.resources().unmap_buffer(self.staging_buffer);
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

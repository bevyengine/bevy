use super::{BufferId, BufferInfo, RenderResource, RenderResourceBinding};
use crate::{
    render_graph::CommandQueue,
    renderer::{BufferUsage, RenderResourceContext},
};
use bevy_ecs::Res;
use std::sync::{Arc, RwLock};

// TODO: Instead of allocating small "exact size" buffers each frame, this should use multiple large shared buffers and probably
// a long-living "cpu mapped" staging buffer. Im punting that for now because I don't know the best way to use wgpu's new async
// buffer mapping yet.
pub struct SharedBuffers {
    render_resource_context: Box<dyn RenderResourceContext>,
    buffers: Arc<RwLock<Vec<BufferId>>>,
    command_queue: Arc<RwLock<CommandQueue>>,
}

impl SharedBuffers {
    pub fn new(render_resource_context: Box<dyn RenderResourceContext>) -> Self {
        Self {
            render_resource_context,
            buffers: Default::default(),
            command_queue: Default::default(),
        }
    }

    pub fn get_buffer<T: RenderResource>(
        &self,
        render_resource: &T,
        buffer_usage: BufferUsage,
    ) -> Option<RenderResourceBinding> {
        if let Some(size) = render_resource.buffer_byte_len() {
            // PERF: this buffer will be slow
            let staging_buffer = self.render_resource_context.create_buffer(BufferInfo {
                size,
                buffer_usage: BufferUsage::COPY_SRC | BufferUsage::MAP_WRITE,
                mapped_at_creation: true,
            });

            self.render_resource_context.write_mapped_buffer(
                staging_buffer,
                0..size as u64,
                &mut |data, _renderer| {
                    render_resource.write_buffer_bytes(data);
                },
            );

            self.render_resource_context.unmap_buffer(staging_buffer);

            let destination_buffer = self.render_resource_context.create_buffer(BufferInfo {
                size,
                buffer_usage: BufferUsage::COPY_DST | buffer_usage,
                ..Default::default()
            });

            let mut command_queue = self.command_queue.write().unwrap();
            command_queue.copy_buffer_to_buffer(
                staging_buffer,
                0,
                destination_buffer,
                0,
                size as u64,
            );

            let mut buffers = self.buffers.write().unwrap();
            buffers.push(staging_buffer);
            buffers.push(destination_buffer);
            Some(RenderResourceBinding::Buffer {
                buffer: destination_buffer,
                range: 0..size as u64,
                dynamic_index: None,
            })
        } else {
            None
        }
    }

    // TODO: remove this when this actually uses shared buffers
    pub fn free_buffers(&self) {
        let mut buffers = self.buffers.write().unwrap();
        for buffer in buffers.drain(..) {
            self.render_resource_context.remove_buffer(buffer)
        }
    }

    pub fn reset_command_queue(&self) -> CommandQueue {
        let mut command_queue = self.command_queue.write().unwrap();
        std::mem::take(&mut *command_queue)
    }
}

// TODO: remove this when this actually uses shared buffers
pub fn free_shared_buffers_system(shared_buffers: Res<SharedBuffers>) {
    shared_buffers.free_buffers();
}

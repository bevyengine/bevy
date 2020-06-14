use super::{BufferId, BufferInfo, RenderResource, RenderResourceBinding};
use crate::{render_resource::BufferUsage, renderer::RenderResourceContext};
use legion::systems::Res;
use std::sync::{Arc, RwLock};

// TODO: Instead of allocating small "exact size" buffers each frame, this should use multiple large shared buffers and probably
// a long-living "cpu mapped" staging buffer. Im punting that for now because I don't know the best way to use wgpu's new async
// buffer mapping yet.
pub struct SharedBuffers {
    render_resource_context: Box<dyn RenderResourceContext>,
    buffers: Arc<RwLock<Vec<BufferId>>>,
}

impl SharedBuffers {
    pub fn new(render_resource_context: Box<dyn RenderResourceContext>) -> Self {
        Self {
            render_resource_context,
            buffers: Default::default(),
        }
    }

    pub fn get_buffer<T: RenderResource>(
        &self,
        render_resource: &T,
        buffer_usage: BufferUsage,
    ) -> Option<RenderResourceBinding> {
        if let Some(size) = render_resource.buffer_byte_len() {
            // PERF: this buffer will be slow
            let buffer = self.render_resource_context.create_buffer_mapped(
                BufferInfo {
                    size,
                    buffer_usage: buffer_usage | BufferUsage::COPY_SRC | BufferUsage::COPY_DST,
                },
                &mut |data, _renderer| {
                    render_resource.write_buffer_bytes(data);
                },
            );
            self.buffers.write().unwrap().push(buffer);
            Some(RenderResourceBinding::Buffer {
                buffer,
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
}

// TODO: remove this when this actually uses shared buffers
pub fn free_shared_buffers_system(shared_buffers: Res<SharedBuffers>) {
    shared_buffers.free_buffers();
}

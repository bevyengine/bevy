use crate::{
    renderer::{BufferId, RenderContext, TextureId},
    texture::Extent3d,
};
use parking_lot::Mutex;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub enum Command {
    CopyBufferToBuffer {
        source_buffer: BufferId,
        source_offset: u64,
        destination_buffer: BufferId,
        destination_offset: u64,
        size: u64,
    },
    CopyBufferToTexture {
        source_buffer: BufferId,
        source_offset: u64,
        source_bytes_per_row: u32,
        destination_texture: TextureId,
        destination_origin: [u32; 3],
        destination_mip_level: u32,
        size: Extent3d,
    },
    // TODO: Frees probably don't need to be queued?
    FreeBuffer(BufferId),
}

#[derive(Debug, Default, Clone)]
pub struct CommandQueue {
    // TODO: this shouldn't really need a mutex. it just needs to be shared on whatever thread it's scheduled on
    queue: Arc<Mutex<Vec<Command>>>,
}

impl CommandQueue {
    fn push(&mut self, command: Command) {
        self.queue.lock().push(command);
    }

    pub fn copy_buffer_to_buffer(
        &mut self,
        source_buffer: BufferId,
        source_offset: u64,
        destination_buffer: BufferId,
        destination_offset: u64,
        size: u64,
    ) {
        self.push(Command::CopyBufferToBuffer {
            source_buffer,
            source_offset,
            destination_buffer,
            destination_offset,
            size,
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub fn copy_buffer_to_texture(
        &mut self,
        source_buffer: BufferId,
        source_offset: u64,
        source_bytes_per_row: u32,
        destination_texture: TextureId,
        destination_origin: [u32; 3],
        destination_mip_level: u32,
        size: Extent3d,
    ) {
        self.push(Command::CopyBufferToTexture {
            source_buffer,
            source_offset,
            source_bytes_per_row,
            destination_texture,
            destination_origin,
            destination_mip_level,
            size,
        });
    }

    pub fn free_buffer(&mut self, buffer: BufferId) {
        self.push(Command::FreeBuffer(buffer));
    }

    pub fn clear(&mut self) {
        self.queue.lock().clear();
    }

    pub fn execute(&mut self, render_context: &mut dyn RenderContext) {
        for command in self.queue.lock().drain(..) {
            match command {
                Command::CopyBufferToBuffer {
                    source_buffer,
                    source_offset,
                    destination_buffer,
                    destination_offset,
                    size,
                } => render_context.copy_buffer_to_buffer(
                    source_buffer,
                    source_offset,
                    destination_buffer,
                    destination_offset,
                    size,
                ),
                Command::CopyBufferToTexture {
                    source_buffer,
                    source_offset,
                    source_bytes_per_row,
                    destination_texture,
                    destination_origin,
                    destination_mip_level,
                    size,
                } => render_context.copy_buffer_to_texture(
                    source_buffer,
                    source_offset,
                    source_bytes_per_row,
                    destination_texture,
                    destination_origin,
                    destination_mip_level,
                    size,
                ),
                Command::FreeBuffer(buffer) => render_context.resources().remove_buffer(buffer),
            }
        }
    }
}

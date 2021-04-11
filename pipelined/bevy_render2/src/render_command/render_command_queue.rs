use crate::{
    render_resource::{BufferId, TextureId},
    renderer::RenderContext,
    texture::Extent3d,
};

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
    CopyTextureToTexture {
        source_texture: TextureId,
        source_origin: [u32; 3],
        source_mip_level: u32,
        destination_texture: TextureId,
        destination_origin: [u32; 3],
        destination_mip_level: u32,
        size: Extent3d,
    },
    CopyTextureToBuffer {
        source_texture: TextureId,
        source_origin: [u32; 3],
        source_mip_level: u32,
        destination_buffer: BufferId,
        destination_offset: u64,
        destination_bytes_per_row: u32,
        size: Extent3d,
    },
    // TODO: Frees probably don't need to be queued?
    FreeBuffer(BufferId),
}

#[derive(Debug, Default, Clone)]
pub struct RenderCommandQueue {
    // TODO: this shouldn't really need a mutex. it just needs to be shared on whatever thread it's
    // scheduled on
    queue: Vec<Command>,
}

impl RenderCommandQueue {
    fn push(&mut self, command: Command) {
        self.queue.push(command);
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

    #[allow(clippy::too_many_arguments)]
    pub fn copy_texture_to_buffer(
        &mut self,
        source_texture: TextureId,
        source_origin: [u32; 3],
        source_mip_level: u32,
        destination_buffer: BufferId,
        destination_offset: u64,
        destination_bytes_per_row: u32,
        size: Extent3d,
    ) {
        self.push(Command::CopyTextureToBuffer {
            source_texture,
            source_origin,
            source_mip_level,
            destination_buffer,
            destination_offset,
            destination_bytes_per_row,
            size,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn copy_texture_to_texture(
        &mut self,
        source_texture: TextureId,
        source_origin: [u32; 3],
        source_mip_level: u32,
        destination_texture: TextureId,
        destination_origin: [u32; 3],
        destination_mip_level: u32,
        size: Extent3d,
    ) {
        self.push(Command::CopyTextureToTexture {
            source_texture,
            source_origin,
            source_mip_level,
            destination_texture,
            destination_origin,
            destination_mip_level,
            size,
        })
    }

    pub fn free_buffer(&mut self, buffer: BufferId) {
        self.push(Command::FreeBuffer(buffer));
    }

    pub fn clear(&mut self) {
        self.queue.clear();
    }

    pub fn extend(&mut self, other: &mut RenderCommandQueue) {
        self.queue.extend(other.queue.drain(..));
    }

    // TODO: Ideally this consumes the queue, but RenderGraph Nodes currently don't have write access to World.
    // This is currently ok because new queues are created every frame
    pub fn execute(&self, render_context: &mut dyn RenderContext) {
        for command in self.queue.iter().cloned() {
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
                Command::CopyTextureToTexture {
                    source_texture,
                    source_origin,
                    source_mip_level,
                    destination_texture,
                    destination_origin,
                    destination_mip_level,
                    size,
                } => render_context.copy_texture_to_texture(
                    source_texture,
                    source_origin,
                    source_mip_level,
                    destination_texture,
                    destination_origin,
                    destination_mip_level,
                    size,
                ),
                Command::CopyTextureToBuffer {
                    source_texture,
                    source_origin,
                    source_mip_level,
                    destination_buffer,
                    destination_offset,
                    destination_bytes_per_row,
                    size,
                } => render_context.copy_texture_to_buffer(
                    source_texture,
                    source_origin,
                    source_mip_level,
                    destination_buffer,
                    destination_offset,
                    destination_bytes_per_row,
                    size,
                ),
                Command::FreeBuffer(buffer) => render_context.resources().remove_buffer(buffer),
            }
        }
    }
}

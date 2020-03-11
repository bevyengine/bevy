use crate::render::render_resource::BufferUsage;

pub enum ResourceInfo {
    Buffer {
        size: u64,
        buffer_usage: BufferUsage,
    },
    InstanceBuffer {
        size: usize,
        count: usize,
        buffer_usage: BufferUsage,
        mesh_id: usize,
    },
    Texture,
    Sampler,
}

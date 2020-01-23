pub type ResourceId = u64;

pub struct Buffer {
    pub id: ResourceId,
    pub size: u64,
    pub buffer_usage: wgpu::BufferUsage,
    // pub layout: Option<
}
pub enum ResourceInfo {
    BufferMapped {
        size: u64,
        buffer_usage: wgpu::BufferUsage,
    },
    Buffer {
        size: u64,
        buffer_usage: wgpu::BufferUsage,
        // pub layout: Option<
    },
    InstanceBuffer {
        size: usize,
        count: usize,
        buffer_usage: wgpu::BufferUsage,
        mesh_id: usize,
        // pub layout: Option<
    },
}
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
}
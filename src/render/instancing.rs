pub struct InstanceBufferInfo {
    pub buffer: wgpu::Buffer,
    pub instance_count: usize,
    pub mesh_id: usize,
}
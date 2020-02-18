use legion::prelude::Entity;
use std::collections::HashMap;

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

pub struct DynamicUniformBufferInfo {
    pub indices: HashMap<usize, Entity>,
    pub offsets: HashMap<Entity, u64>,
    pub capacity: u64,
    pub count: u64,
    pub size: u64,
}

impl DynamicUniformBufferInfo {
    pub fn new() -> Self {
        DynamicUniformBufferInfo {
            capacity: 0,
            count: 0,
            indices: HashMap::new(),
            offsets: HashMap::new(),
            size: 0,
        }
    }
}

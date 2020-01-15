use crate::render::UniformBuffer;
use std::collections::HashMap;

pub struct RenderGraphData {
    pub swap_chain_descriptor: wgpu::SwapChainDescriptor,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface,
    textures: HashMap<String, wgpu::TextureView>,
    samplers: HashMap<String, wgpu::Sampler>,
    uniform_buffers: HashMap<String, UniformBuffer>,
    bind_group_layouts: HashMap<String, wgpu::BindGroupLayout>,
}

impl RenderGraphData {
    pub fn new(
        device: wgpu::Device,
        swap_chain_descriptor: wgpu::SwapChainDescriptor,
        queue: wgpu::Queue,
        surface: wgpu::Surface,
    ) -> Self {
        RenderGraphData {
            textures: HashMap::new(),
            samplers: HashMap::new(),
            uniform_buffers: HashMap::new(),
            bind_group_layouts: HashMap::new(),
            device,
            swap_chain_descriptor,
            queue,
            surface,
        }
    }

    pub fn set_uniform_buffer(&mut self, name: &str, uniform_buffer: UniformBuffer) {
        self.uniform_buffers
            .insert(name.to_string(), uniform_buffer);
    }

    pub fn get_uniform_buffer(&self, name: &str) -> Option<&UniformBuffer> {
        self.uniform_buffers.get(name)
    }

    pub fn set_bind_group_layout(&mut self, name: &str, bind_group_layout: wgpu::BindGroupLayout) {
        self.bind_group_layouts
            .insert(name.to_string(), bind_group_layout);
    }

    pub fn get_bind_group_layout(&self, name: &str) -> Option<&wgpu::BindGroupLayout> {
        self.bind_group_layouts.get(name)
    }

    pub fn set_texture(&mut self, name: &str, texture: wgpu::TextureView) {
        self.textures.insert(name.to_string(), texture);
    }

    pub fn get_texture(&self, name: &str) -> Option<&wgpu::TextureView> {
        self.textures.get(name)
    }

    pub fn set_sampler(&mut self, name: &str, sampler: wgpu::Sampler) {
        self.samplers.insert(name.to_string(), sampler);
    }

    pub fn get_sampler(&self, name: &str) -> Option<&wgpu::Sampler> {
        self.samplers.get(name)
    }
}

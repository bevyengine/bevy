use crate::{asset::Mesh, legion::prelude::*, render::render_graph_2::{RenderGraph, Buffer, ResourceId}};

pub trait Renderer {
    fn initialize(&mut self, world: &mut World);
    fn resize(&mut self, world: &mut World, width: u32, height: u32);
    fn process_render_graph(&mut self, render_graph: &RenderGraph, world: &mut World);
    // TODO: swap out wgpu::BufferUsage for custom type
    fn create_buffer_with_data(&mut self, data: &[u8], buffer_usage: wgpu::BufferUsage) -> Buffer;
    fn free_buffer(&mut self, id: ResourceId) -> Buffer;
}

pub trait RenderPass {
    fn set_index_buffer(&mut self, buffer: &wgpu::Buffer, offset: wgpu::BufferAddress);
}
use crate::{asset::Mesh, legion::prelude::*, render::render_graph_2::RenderGraph};

pub trait Renderer {
    fn initialize(&mut self, world: &mut World);
    fn resize(&mut self, world: &mut World, width: u32, height: u32);
    fn process_render_graph(&mut self, render_graph: &RenderGraph, world: &mut World);
    fn load_mesh(&mut self, asset_id: usize, mesh: &Mesh);
}

pub trait RenderPass {
    fn set_index_buffer(&mut self, buffer: &wgpu::Buffer, offset: wgpu::BufferAddress);
}
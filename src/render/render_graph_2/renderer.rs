use crate::{legion::prelude::*, render::render_graph_2::{RenderGraph, BufferInfo, PipelineDescriptor}};
use std::ops::Range;

pub trait Renderer {
    fn initialize(&mut self, world: &mut World);
    fn resize(&mut self, world: &mut World, width: u32, height: u32);
    fn process_render_graph(&mut self, render_graph: &RenderGraph, world: &mut World);
    // TODO: swap out wgpu::BufferUsage for custom type
    fn create_buffer_with_data(&mut self, name: &str, data: &[u8], buffer_usage: wgpu::BufferUsage);
    fn remove_buffer(&mut self, name: &str);
    fn get_buffer_info(&self, name: &str) -> Option<&BufferInfo>;
}

pub trait RenderPass {
    fn get_renderer(&mut self) -> &mut dyn Renderer;
    fn get_pipeline_descriptor(&self) -> &PipelineDescriptor;
    fn set_index_buffer(&mut self, name: &str, offset: u64);
    fn set_vertex_buffer(&mut self, start_slot: u32, name: &str, offset: u64);
    fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>);
}
use crate::{legion::prelude::*, render::render_graph_2::{RenderGraph, ResourceInfo, PipelineDescriptor}};
use std::ops::Range;

pub trait Renderer {
    fn initialize(&mut self, world: &mut World, render_graph: &mut RenderGraph);
    fn resize(&mut self, world: &mut World, render_graph: &mut RenderGraph, width: u32, height: u32);
    fn process_render_graph(&mut self, render_graph: &mut RenderGraph, world: &mut World);
    // TODO: swap out wgpu::BufferUsage for non-wgpu type
    fn create_buffer_with_data(&mut self, name: &str, data: &[u8], buffer_usage: wgpu::BufferUsage);
    fn create_buffer(&mut self, name: &str, size: u64, buffer_usage: wgpu::BufferUsage);
    fn remove_buffer(&mut self, name: &str);
    fn get_resource_info(&self, name: &str) -> Option<&ResourceInfo>;
}

pub trait RenderPass {
    // TODO: consider using static dispatch for the renderer: Renderer<WgpuBackend>. compare compile times
    fn get_renderer(&mut self) -> &mut dyn Renderer;
    fn get_pipeline_descriptor(&self) -> &PipelineDescriptor;
    fn set_index_buffer(&mut self, name: &str, offset: u64);
    fn set_vertex_buffer(&mut self, start_slot: u32, name: &str, offset: u64);
    fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>);
    fn setup_bind_groups(&mut self, entity: Option<&Entity>);
}
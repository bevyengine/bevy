use legion::world::World;
use wgpu::{Buffer, CommandEncoder, Device, SwapChainDescriptor, SwapChainOutput};
use crate::render::{RenderResources, RenderGraphData};


pub trait Pipeline {
    fn render(&mut self, device: &Device, frame: &SwapChainOutput, encoder: &mut CommandEncoder, world: &mut World, render_resources: &RenderResources);
    fn resize(&mut self, device: &Device, frame: &SwapChainDescriptor);
    fn get_camera_uniform_buffer(&self) -> Option<&Buffer>;
}

pub trait PipelineNew {
    fn initialize(&mut self, render_graph: &mut RenderGraphData, world: &mut World);
    fn render(&mut self, render_graph: &RenderGraphData, pass: &mut wgpu::RenderPass, frame: &SwapChainOutput, world: &mut World);
    fn resize(&mut self, render_graph: &RenderGraphData);
    fn get_pipeline(&self) -> &wgpu::RenderPipeline;
}
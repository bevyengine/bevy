use legion::world::World;
use wgpu::SwapChainOutput;
use crate::render::RenderGraphData;

pub trait Pipeline {
    fn initialize(&mut self, render_graph: &mut RenderGraphData, world: &mut World);
    fn render(&mut self, render_graph: &RenderGraphData, pass: &mut wgpu::RenderPass, frame: &SwapChainOutput, world: &mut World);
    fn resize(&mut self, render_graph: &RenderGraphData);
    fn get_pipeline(&self) -> &wgpu::RenderPipeline;
}
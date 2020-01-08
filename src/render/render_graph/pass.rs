use crate::render::render_graph::RenderGraphData;
use legion::world::World;

pub trait Pass {
    fn initialize(&self, render_graph: &mut RenderGraphData);
    fn begin<'a>(&mut self, render_graph: &mut RenderGraphData, world: &mut World, encoder: &'a mut wgpu::CommandEncoder, frame: &'a wgpu::SwapChainOutput) -> Option<wgpu::RenderPass<'a>>;
    fn should_repeat(&self) -> bool;
    fn resize(&self, render_graph: &mut RenderGraphData);
}
use crate::render::RenderGraphData;
use legion::world::World;

pub trait RenderResourceManager {
    fn initialize(&self, render_graph: &mut RenderGraphData, world: &mut World);
    fn update<'a>(
        &mut self,
        render_graph: &mut RenderGraphData,
        encoder: &'a mut wgpu::CommandEncoder,
        world: &mut World,
    );
    fn resize<'a>(
        &self,
        render_graph: &mut RenderGraphData,
        encoder: &'a mut wgpu::CommandEncoder,
        world: &mut World,
    );
}

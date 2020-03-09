use super::Renderer;
use crate::{
    asset::Handle,
    render::render_graph::{pipeline::PipelineDescriptor, RenderPass},
};
use legion::prelude::World;

// A set of draw calls. ex: get + draw meshes, get + draw instanced meshes, draw ui meshes, etc

// TODO: consider swapping out dyn RenderPass for explicit WgpuRenderPass type to avoid dynamic dispatch
pub type DrawTarget = fn(
    world: &World,
    render_pass: &mut dyn RenderPass,
    pipeline_handle: Handle<PipelineDescriptor>,
);

pub trait NewDrawTarget {
    fn draw(
        &self,
        world: &World,
        render_pass: &mut dyn RenderPass,
        pipeline_handle: Handle<PipelineDescriptor>,
    );
    fn setup(
        &mut self,
        world: &World,
        renderer: &mut dyn Renderer,
        pipeline_handle: Handle<PipelineDescriptor>,
    );
    fn get_name(&self) -> String;
}

use super::Renderer;
use crate::{
    asset::Handle,
    render::render_graph::{pipeline::PipelineDescriptor, RenderPass},
};
use legion::prelude::{Resources, World};

// A set of draw calls. ex: get + draw meshes, get + draw instanced meshes, draw ui meshes, etc
pub trait DrawTarget {
    fn draw(
        &self,
        world: &World,
        resources: &Resources,
        render_pass: &mut dyn RenderPass,
        pipeline_handle: Handle<PipelineDescriptor>,
    );
    fn setup(
        &mut self,
        world: &World,
        world: &Resources,
        renderer: &mut dyn Renderer,
        pipeline_handle: Handle<PipelineDescriptor>,
    );
    fn get_name(&self) -> String;
}

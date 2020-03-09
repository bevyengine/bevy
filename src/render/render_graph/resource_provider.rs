use crate::render::render_graph::Renderer;
use legion::prelude::*;

pub trait ResourceProvider {
    fn initialize(&mut self, renderer: &mut dyn Renderer, world: &mut World, resources: &Resources);
    fn update(&mut self, renderer: &mut dyn Renderer, world: &mut World, resources: &Resources);
    fn resize(
        &mut self,
        renderer: &mut dyn Renderer,
        world: &mut World,
        resources: &Resources,
        width: u32,
        height: u32,
    );
}

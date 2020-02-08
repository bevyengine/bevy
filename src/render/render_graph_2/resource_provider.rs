use crate::render::render_graph_2::Renderer;
use legion::prelude::*;

pub trait ResourceProvider {
    fn initialize(&mut self, renderer: &mut dyn Renderer, world: &mut World);
    fn update(&mut self, renderer: &mut dyn Renderer, world: &mut World);
    fn resize(&mut self, renderer: &mut dyn Renderer, world: &mut World, width: u32, height: u32);
}

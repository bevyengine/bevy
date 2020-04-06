use crate::renderer::Renderer;
use legion::prelude::*;

pub trait ResourceProvider {
    fn initialize(
        &mut self,
        _renderer: &mut dyn Renderer,
        _world: &mut World,
        _resources: &Resources,
    ) {
    }
    fn update(&mut self, _renderer: &mut dyn Renderer, _world: &mut World, _resources: &Resources) {
    }
    fn finish_update(
        &mut self,
        _renderer: &mut dyn Renderer,
        _world: &mut World,
        _resources: &Resources,
    ) {
    }
}

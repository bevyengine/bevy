use crate::{renderer_2::RenderContext};
use legion::prelude::*;

pub trait ResourceProvider {
    fn initialize(
        &mut self,
        _renderer: &mut dyn RenderContext,
        _world: &mut World,
        _resources: &Resources,
    ) {
    }
    fn update(&mut self, _render_context: &mut dyn RenderContext, _world: &mut World, _resources: &Resources) {
    }
    fn finish_update(
        &mut self,
        _render_context: &mut dyn RenderContext,
        _world: &mut World,
        _resources: &Resources,
    ) {
    }
}

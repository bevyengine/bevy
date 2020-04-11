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

    // TODO: make this read-only
    fn update(&mut self, _render_context: &mut dyn RenderContext, _world: &mut World, _resources: &Resources) {
    }

    // TODO: remove this
    fn finish_update(
        &mut self,
        _render_context: &mut dyn RenderContext,
        _world: &mut World,
        _resources: &Resources,
    ) {
    }

    /// Runs after resources have been created on the gpu. In general systems here write gpu-related resources back to entities in this step  
    fn post_update(&mut self, _render_context: &dyn RenderContext, _world: &mut World, _resources: &Resources) {

    }

}

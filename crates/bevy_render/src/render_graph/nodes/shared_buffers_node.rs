use crate::{
    render_graph::{Node, ResourceSlots},
    renderer::{RenderContext, SharedBuffers},
};
use bevy_ecs::world::World;

#[derive(Default)]
pub struct SharedBuffersNode;

impl Node for SharedBuffersNode {
    fn update(
        &mut self,
        world: &World,
        render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        let shared_buffers = world.get_resource::<SharedBuffers>().unwrap();
        shared_buffers.apply(render_context);
    }
}

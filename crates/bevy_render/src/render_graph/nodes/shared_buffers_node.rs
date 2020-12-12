use crate::{
    render_graph::{Node, ResourceSlots},
    renderer::{RenderContext, SharedBuffers},
};
use bevy_ecs::{Resources, World};

#[derive(Debug, Default)]
pub struct SharedBuffersNode;

impl Node for SharedBuffersNode {
    fn update(
        &mut self,
        _world: &World,
        resources: &Resources,
        render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        let mut shared_buffers = resources.get_mut::<SharedBuffers>().unwrap();
        shared_buffers.apply(render_context);
    }
}

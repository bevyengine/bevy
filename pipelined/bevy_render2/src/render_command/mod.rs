mod render_command_queue;
pub use render_command_queue::*;

use crate::{
    render_graph::{Node, RenderGraph, ResourceSlots},
    renderer::RenderContext,
    RenderStage,
};
use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;

#[derive(Default)]
pub struct RenderCommandPlugin;

impl Plugin for RenderCommandPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RenderCommandQueue>();
        let render_app = app.sub_app_mut(0);
        render_app.add_system_to_stage(RenderStage::Extract, extract_render_commands.system());
        let mut graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();
        graph.add_node("render_command_queue", RenderCommandQueueNode);
    }
}

fn extract_render_commands(
    mut commands: Commands,
    mut render_command_queue: ResMut<RenderCommandQueue>,
) {
    let mut queue = RenderCommandQueue::default();
    queue.extend(&mut render_command_queue);
    commands.insert_resource(queue);
}

pub struct RenderCommandQueueNode;

impl Node for RenderCommandQueueNode {
    fn update(
        &mut self,
        world: &World,
        render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        let queue = world.get_resource::<RenderCommandQueue>().unwrap();
        queue.execute(render_context);
    }
}

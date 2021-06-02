mod render_command_queue;
pub use render_command_queue::*;

use crate::{
    render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext},
    renderer::RenderContext,
    RenderStage,
};
use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;

#[derive(Default)]
pub struct RenderCommandPlugin;

impl RenderCommandPlugin {
    pub const RENDER_COMMAND_QUEUE_NODE: &'static str = "render_command_queue";
}

impl Plugin for RenderCommandPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RenderCommandQueue>();
        let render_app = app.sub_app_mut(0);
        render_app.add_system_to_stage(RenderStage::Extract, extract_render_commands.system());
        let mut graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();
        graph.add_node(Self::RENDER_COMMAND_QUEUE_NODE, RenderCommandQueueNode);
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
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut dyn RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let queue = world.get_resource::<RenderCommandQueue>().unwrap();
        queue.execute(render_context);
        Ok(())
    }
}

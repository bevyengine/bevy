use bevy_ecs::world::World;
use bevy_render::{
    render_graph::{Node, NodeRunError, RenderGraphContext, RunSubGraphs, SlotValues},
    renderer::RenderContext,
};

pub struct ClearPassDriverNode;

impl Node for ClearPassDriverNode {
    fn queue_graphs(&self, graph: &RenderGraphContext, world: &World) -> Result<bevy_render::render_graph::RunSubGraphs, NodeRunError> {
        
        let mut sub_graph_runs = RunSubGraphs::default();
        sub_graph_runs.run(crate::clear_graph::NAME, SlotValues::empty());

        Ok(sub_graph_runs)
    }
}

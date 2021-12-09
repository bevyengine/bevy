use bevy_ecs::world::World;
use bevy_render2::{
    render_graph::{Node, NodeRunError, RenderGraphContext},
    renderer::RenderContext,
};

pub struct ClearPassDriverNode;

impl Node for ClearPassDriverNode {
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        _render_context: &mut RenderContext,
        _world: &World,
    ) -> Result<(), NodeRunError> {
        graph.run_sub_graph(crate::clear_graph::NAME, vec![])?;

        Ok(())
    }
}

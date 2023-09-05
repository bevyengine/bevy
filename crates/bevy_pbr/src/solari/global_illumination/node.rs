use super::{
    pipelines::SolariGlobalIlluminationPipelineIds,
    view_resources::SolariGlobalIlluminationBindGroups,
};
use bevy_ecs::{query::QueryItem, world::World};
use bevy_render::{
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    renderer::RenderContext,
};

#[derive(Default)]
pub struct SolariGlobalIlluminationNode;

impl ViewNode for SolariGlobalIlluminationNode {
    type ViewQuery = (
        &'static SolariGlobalIlluminationPipelineIds,
        &'static SolariGlobalIlluminationBindGroups,
    );

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (pipeline_ids, bind_groups): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        todo!()
    }
}

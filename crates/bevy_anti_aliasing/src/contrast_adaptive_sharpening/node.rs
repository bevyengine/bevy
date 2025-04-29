use std::sync::Mutex;

use crate::contrast_adaptive_sharpening::ViewCasPipeline;
use bevy_ecs::prelude::*;
use bevy_render::{
    extract_component::DynamicUniformIndex,
    frame_graph::FrameGraph,
    render_graph::{Node, NodeRunError, RenderGraphContext},
    render_resource::{BindGroup, BufferId, TextureViewId},
    view::{ExtractedView, ViewTarget},
};

use super::CasUniform;

pub struct CasNode {
    query: QueryState<
        (
            &'static ViewTarget,
            &'static ViewCasPipeline,
            &'static DynamicUniformIndex<CasUniform>,
        ),
        With<ExtractedView>,
    >,
    cached_bind_group: Mutex<Option<(BufferId, TextureViewId, BindGroup)>>,
}

impl FromWorld for CasNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
            cached_bind_group: Mutex::new(None),
        }
    }
}

impl Node for CasNode {
    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        _frame_graph: &mut FrameGraph,
        _world: &World,
    ) -> Result<(), NodeRunError> {
        //todo
        Ok(())
    }
}

use bevy_ecs::{query::QueryItem, world::World};
use bevy_render::{
    frame_graph::FrameGraph,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    view::{Msaa, ViewTarget},
};

use crate::prepass::ViewPrepassTextures;

use super::{pipeline::MotionBlurPipelineId, MotionBlurUniform};

#[derive(Default)]
pub struct MotionBlurNode;

impl ViewNode for MotionBlurNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static MotionBlurPipelineId,
        &'static ViewPrepassTextures,
        &'static MotionBlurUniform,
        &'static Msaa,
    );
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        _frame_graph: &mut FrameGraph,
        (_view_target, _pipeline_id, _prepass_textures, _motion_blur, _msaa): QueryItem<
            Self::ViewQuery,
        >,
        _world: &World,
    ) -> Result<(), NodeRunError> {
        Ok(())
    }
}

use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::{
    camera::ExtractedCamera,
    frame_graph::FrameGraph,
    render_graph::{NodeRunError, RenderGraphContext, RenderLabel, ViewNode},
    view::{ViewDepthTexture, ViewTarget, ViewUniformOffset},
};

use super::OitResolvePipelineId;

/// Render label for the OIT resolve pass.
#[derive(RenderLabel, Debug, Clone, Hash, PartialEq, Eq)]
pub struct OitResolvePass;

/// The node that executes the OIT resolve pass.
#[derive(Default)]
pub struct OitResolveNode;
impl ViewNode for OitResolveNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static ViewUniformOffset,
        &'static OitResolvePipelineId,
        &'static ViewDepthTexture,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        frame_graph: &mut FrameGraph,
        (_camera, _view_target, _view_uniform, _oit_resolve_pipeline_id, depth): QueryItem<
            Self::ViewQuery,
        >,
        _world: &World,
    ) -> Result<(), NodeRunError> {
        Ok(())
    }
}

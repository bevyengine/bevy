use bevy_ecs::prelude::*;
use bevy_render::{
    camera::ExtractedCamera,
    frame_graph::FrameGraph,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    view::{ExtractedView, ViewDepthTexture, ViewTarget},
};
#[cfg(feature = "trace")]
use tracing::info_span;

#[derive(Default)]
pub struct MainTransparentPass2dNode {}

impl ViewNode for MainTransparentPass2dNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ExtractedView,
        &'static ViewTarget,
        &'static ViewDepthTexture,
    );

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        _frame_graph: &mut FrameGraph,
        (_camera, _view, _target, _depth): bevy_ecs::query::QueryItem<'w, Self::ViewQuery>,
        _world: &'w World,
    ) -> Result<(), NodeRunError> {
        Ok(())
    }
}

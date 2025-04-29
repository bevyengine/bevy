use crate::skybox::{SkyboxBindGroup, SkyboxPipelineId};
use bevy_ecs::{prelude::World, query::QueryItem};
use bevy_render::{
    camera::ExtractedCamera,
    frame_graph::FrameGraph,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    view::{ExtractedView, ViewDepthTexture, ViewTarget, ViewUniformOffset},
};
#[cfg(feature = "trace")]
use tracing::info_span;

/// A [`bevy_render::render_graph::Node`] that runs the [`Opaque3d`] and [`AlphaMask3d`]
/// [`ViewBinnedRenderPhases`]s.
#[derive(Default)]
pub struct MainOpaquePass3dNode;
impl ViewNode for MainOpaquePass3dNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ExtractedView,
        &'static ViewTarget,
        &'static ViewDepthTexture,
        Option<&'static SkyboxPipelineId>,
        Option<&'static SkyboxBindGroup>,
        &'static ViewUniformOffset,
    );

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        _render_context: &mut FrameGraph,
        (
            _camera,
            _extracted_view,
            _target,
            _depth,
            _skybox_pipeline,
            _skybox_bind_group,
            _view_uniform_offset,
        ): QueryItem<'w, Self::ViewQuery>,
        _world: &'w World,
    ) -> Result<(), NodeRunError> {
        Ok(())
    }
}

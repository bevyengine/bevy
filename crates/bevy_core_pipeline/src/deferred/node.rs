use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::experimental::occlusion_culling::OcclusionCulling;
use bevy_render::frame_graph::FrameGraph;
use bevy_render::render_graph::ViewNode;

use bevy_render::view::{ExtractedView, NoIndirectDrawing};
use bevy_render::{
    camera::ExtractedCamera,
    render_graph::{NodeRunError, RenderGraphContext},
    view::ViewDepthTexture,
};
#[cfg(feature = "trace")]
use tracing::info_span;

use crate::prepass::ViewPrepassTextures;

/// The phase of the deferred prepass that draws meshes that were visible last
/// frame.
///
/// If occlusion culling isn't in use, this prepass simply draws all meshes.
///
/// Like all prepass nodes, this is inserted before the main pass in the render
/// graph.
#[derive(Default)]
pub struct EarlyDeferredGBufferPrepassNode;

impl ViewNode for EarlyDeferredGBufferPrepassNode {
    type ViewQuery = <LateDeferredGBufferPrepassNode as ViewNode>::ViewQuery;

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        _frame_graph: &mut FrameGraph,
        _view_query: QueryItem<'w, Self::ViewQuery>,
        _world: &'w World,
    ) -> Result<(), NodeRunError> {
        Ok(())
    }
}

/// The phase of the prepass that runs after occlusion culling against the
/// meshes that were visible last frame.
///
/// If occlusion culling isn't in use, this is a no-op.
///
/// Like all prepass nodes, this is inserted before the main pass in the render
/// graph.
#[derive(Default)]
pub struct LateDeferredGBufferPrepassNode;

impl ViewNode for LateDeferredGBufferPrepassNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ExtractedView,
        &'static ViewDepthTexture,
        &'static ViewPrepassTextures,
        Has<OcclusionCulling>,
        Has<NoIndirectDrawing>,
    );

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        _frame_graph: &mut FrameGraph,
        _view_query: QueryItem<'w, Self::ViewQuery>,
        _world: &'w World,
    ) -> Result<(), NodeRunError> {
        Ok(())
    }
}

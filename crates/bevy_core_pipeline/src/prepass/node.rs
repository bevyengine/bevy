use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::{
    camera::ExtractedCamera,
    experimental::occlusion_culling::OcclusionCulling,
    frame_graph::FrameGraph,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    view::{ExtractedView, NoIndirectDrawing, ViewDepthTexture, ViewUniformOffset},
};
#[cfg(feature = "trace")]
use tracing::info_span;

use crate::skybox::prepass::{RenderSkyboxPrepassPipeline, SkyboxPrepassBindGroup};

use super::{DeferredPrepass, PreviousViewUniformOffset, ViewPrepassTextures};

/// The phase of the prepass that draws meshes that were visible last frame.
///
/// If occlusion culling isn't in use, this prepass simply draws all meshes.
///
/// Like all prepass nodes, this is inserted before the main pass in the render
/// graph.
#[derive(Default)]
pub struct EarlyPrepassNode;

impl ViewNode for EarlyPrepassNode {
    type ViewQuery = <LatePrepassNode as ViewNode>::ViewQuery;

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
pub struct LatePrepassNode;

impl ViewNode for LatePrepassNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ExtractedView,
        &'static ViewDepthTexture,
        &'static ViewPrepassTextures,
        &'static ViewUniformOffset,
        Option<&'static DeferredPrepass>,
        Option<&'static RenderSkyboxPrepassPipeline>,
        Option<&'static SkyboxPrepassBindGroup>,
        Option<&'static PreviousViewUniformOffset>,
        Has<OcclusionCulling>,
        Has<NoIndirectDrawing>,
        Has<DeferredPrepass>,
    );

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        _frame_graph: &mut FrameGraph,
        _query: QueryItem<'w, Self::ViewQuery>,
        _world: &'w World,
    ) -> Result<(), NodeRunError> {
        Ok(())
    }
}

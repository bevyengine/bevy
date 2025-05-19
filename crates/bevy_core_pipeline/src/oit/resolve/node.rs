use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::{
    camera::ExtractedCamera,
    frame_graph::FrameGraph,
    render_graph::{NodeRunError, RenderGraphContext, RenderLabel, ViewNode},
    render_resource::PipelineCache,
    view::{ViewDepthTexture, ViewTarget, ViewUniformOffset},
};

use super::{OitResolveBindGroup, OitResolvePipeline, OitResolvePipelineId};

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
        (camera, view_target, view_uniform, oit_resolve_pipeline_id, depth): QueryItem<
            Self::ViewQuery,
        >,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let Some(resolve_pipeline) = world.get_resource::<OitResolvePipeline>() else {
            return Ok(());
        };

        let pipeline_cache = world.resource::<PipelineCache>();
        let bind_group = world.resource::<OitResolveBindGroup>();
        let Some(_) = pipeline_cache.get_render_pipeline(oit_resolve_pipeline_id.0) else {
            return Ok(());
        };

        let depth_bind_group_handle = frame_graph
            .create_bind_group_handle_builder(
                Some("oit_resolve_depth_bind_group".into()),
                &resolve_pipeline.oit_depth_bind_group_layout,
            )
            .add_helper(0, &depth.texture)
            .build();

        let mut pass_builder = frame_graph.create_pass_builder("oit_resolve_node");

        let color_attachment = view_target.get_color_attachment(&mut pass_builder);

        pass_builder
            .create_render_pass_builder("oit_resolve_pass")
            .add_color_attachment(color_attachment)
            .set_camera_viewport(camera.viewport.clone())
            .set_render_pipeline(oit_resolve_pipeline_id.0)
            .set_raw_bind_group(0, Some(bind_group), &[view_uniform.offset])
            .set_bind_group_handle(1, &depth_bind_group_handle, &[])
            .draw(0..3, 0..1);

        Ok(())
    }
}

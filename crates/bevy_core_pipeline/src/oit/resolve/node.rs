use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::{
    camera::ExtractedCamera,
    render_graph::{NodeRunError, RenderGraphContext, RenderLabel, ViewNode},
    render_resource::{BindGroupEntries, PipelineCache, RenderPassDescriptor},
    renderer::RenderContext,
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
        render_context: &mut RenderContext,
        (camera, view_target, view_uniform, oit_resolve_pipeline_id, depth): QueryItem<
            Self::ViewQuery,
        >,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let Some(resolve_pipeline) = world.get_resource::<OitResolvePipeline>() else {
            return Ok(());
        };

        // resolve oit
        // sorts the layers and renders the final blended color to the screen
        {
            let pipeline_cache = world.resource::<PipelineCache>();
            let bind_group = world.resource::<OitResolveBindGroup>();
            let Some(pipeline) = pipeline_cache.get_render_pipeline(oit_resolve_pipeline_id.0)
            else {
                return Ok(());
            };

            let depth_bind_group = render_context.render_device().create_bind_group(
                "oit_resolve_depth_bind_group",
                &resolve_pipeline.oit_depth_bind_group_layout,
                &BindGroupEntries::single(depth.view()),
            );

            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("oit_resolve_pass"),
                color_attachments: &[Some(view_target.get_color_attachment())],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if let Some(viewport) = camera.viewport.as_ref() {
                render_pass.set_camera_viewport(viewport);
            }

            render_pass.set_render_pipeline(pipeline);
            render_pass.set_bind_group(0, bind_group, &[view_uniform.offset]);
            render_pass.set_bind_group(1, &depth_bind_group, &[]);

            render_pass.draw(0..3, 0..1);
        }

        Ok(())
    }
}

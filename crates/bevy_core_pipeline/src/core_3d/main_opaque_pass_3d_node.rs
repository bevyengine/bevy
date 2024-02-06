use crate::{
    core_3d::Opaque3d,
    skybox::{SkyboxBindGroup, SkyboxPipelineId},
};
use bevy_ecs::{prelude::World, query::QueryItem};
use bevy_render::{
    camera::ExtractedCamera,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_phase::RenderPhase,
    render_resource::{PipelineCache, RenderPassDescriptor, StoreOp},
    renderer::RenderContext,
    view::{ViewDepthTexture, ViewTarget, ViewUniformOffset},
};
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;

use super::AlphaMask3d;

/// A [`bevy_render::render_graph::Node`] that runs the [`Opaque3d`] and [`AlphaMask3d`] [`RenderPhase`].
#[derive(Default)]
pub struct MainOpaquePass3dNode;
impl ViewNode for MainOpaquePass3dNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static RenderPhase<Opaque3d>,
        &'static RenderPhase<AlphaMask3d>,
        &'static ViewTarget,
        &'static ViewDepthTexture,
        Option<&'static SkyboxPipelineId>,
        Option<&'static SkyboxBindGroup>,
        &'static ViewUniformOffset,
    );

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (
            camera,
            opaque_phase,
            alpha_mask_phase,
            target,
            depth,
            skybox_pipeline,
            skybox_bind_group,
            view_uniform_offset,
        ): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        // Run the opaque pass, sorted by pipeline key and mesh id to greatly improve batching.
        // NOTE: Scoped to drop the mutable borrow of render_context
        #[cfg(feature = "trace")]
        let _main_opaque_pass_3d_span = info_span!("main_opaque_pass_3d").entered();

        // Setup render pass
        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("main_opaque_pass_3d"),
            color_attachments: &[Some(target.get_color_attachment())],
            depth_stencil_attachment: Some(depth.get_attachment(StoreOp::Store)),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        if let Some(viewport) = camera.viewport.as_ref() {
            render_pass.set_camera_viewport(viewport);
        }

        let view_entity = graph.view_entity();

        // Opaque draws
        opaque_phase.render(&mut render_pass, world, view_entity);

        // Alpha draws
        if !alpha_mask_phase.items.is_empty() {
            alpha_mask_phase.render(&mut render_pass, world, view_entity);
        }

        // Draw the skybox using a fullscreen triangle
        if let (Some(skybox_pipeline), Some(SkyboxBindGroup(skybox_bind_group))) =
            (skybox_pipeline, skybox_bind_group)
        {
            let pipeline_cache = world.resource::<PipelineCache>();
            if let Some(pipeline) = pipeline_cache.get_render_pipeline(skybox_pipeline.0) {
                render_pass.set_render_pipeline(pipeline);
                render_pass.set_bind_group(
                    0,
                    &skybox_bind_group.0,
                    &[view_uniform_offset.offset, skybox_bind_group.1],
                );
                render_pass.draw(0..3, 0..1);
            }
        }

        Ok(())
    }
}

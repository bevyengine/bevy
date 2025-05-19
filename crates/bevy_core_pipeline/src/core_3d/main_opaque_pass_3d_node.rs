use crate::skybox::{SkyboxBindGroup, SkyboxPipelineId};
use bevy_ecs::{prelude::World, query::QueryItem};
use bevy_render::{
    camera::ExtractedCamera,
    frame_graph::{FrameGraph, PassBuilder},
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_phase::{TrackedRenderPass, ViewBinnedRenderPhases},
    render_resource::{PipelineCache, StoreOp},
    renderer::RenderDevice,
    view::{ExtractedView, ViewDepthTexture, ViewTarget, ViewUniformOffset},
};
use tracing::error;
#[cfg(feature = "trace")]
use tracing::info_span;

use super::{AlphaMask3d, Opaque3d};

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
        graph: &mut RenderGraphContext,
        frame_graph: &mut FrameGraph,
        (
            camera,
            extracted_view,
            target,
            depth,
            skybox_pipeline,
            skybox_bind_group,
            view_uniform_offset,
        ): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let (Some(opaque_phases), Some(alpha_mask_phases)) = (
            world.get_resource::<ViewBinnedRenderPhases<Opaque3d>>(),
            world.get_resource::<ViewBinnedRenderPhases<AlphaMask3d>>(),
        ) else {
            return Ok(());
        };

        let (Some(opaque_phase), Some(alpha_mask_phase)) = (
            opaque_phases.get(&extracted_view.retained_view_entity),
            alpha_mask_phases.get(&extracted_view.retained_view_entity),
        ) else {
            return Ok(());
        };

        let mut pass_builder =
            PassBuilder::new(frame_graph.create_pass_node_bulder("main_opaque_pass_3d_node"));

        let color_attachment = target.get_color_attachment(&mut pass_builder);
        let depth_stencil_attachment =
            depth.get_depth_stencil_attachment(&mut pass_builder, StoreOp::Store);

        let mut builder = pass_builder.create_render_pass_builder("main_opaque_pass_3d");

        builder
            .add_color_attachment(color_attachment)
            .set_depth_stencil_attachment(depth_stencil_attachment)
            .set_camera_viewport(camera.viewport.clone());

        let view_entity = graph.view_entity();
        let render_device = world.resource::<RenderDevice>();
        let mut tracked_render_pass = TrackedRenderPass::new(&render_device, builder);

        // Opaque draws
        if !opaque_phase.is_empty() {
            #[cfg(feature = "trace")]
            let _opaque_main_pass_3d_span = info_span!("opaque_main_pass_3d").entered();
            if let Err(err) = opaque_phase.render(&mut tracked_render_pass, world, view_entity) {
                error!("Error encountered while rendering the opaque phase {err:?}");
            }
        }

        if !alpha_mask_phase.is_empty() {
            #[cfg(feature = "trace")]
            let _alpha_mask_main_pass_3d_span = info_span!("alpha_mask_main_pass_3d").entered();
            if let Err(err) = alpha_mask_phase.render(&mut tracked_render_pass, world, view_entity)
            {
                error!("Error encountered while rendering the alpha mask phase {err:?}");
            }
        }

        // Skybox draw using a fullscreen triangle
        if let (Some(skybox_pipeline), Some(SkyboxBindGroup(skybox_bind_group))) =
            (skybox_pipeline, skybox_bind_group)
        {
            let pipeline_cache = world.resource::<PipelineCache>();
            if let Some(_) = pipeline_cache.get_render_pipeline(skybox_pipeline.0) {
                tracked_render_pass.set_render_pipeline(skybox_pipeline.0);
                tracked_render_pass.set_bind_group_handle(
                    0,
                    &skybox_bind_group.0,
                    &[view_uniform_offset.offset, skybox_bind_group.1],
                );
                tracked_render_pass.draw(0..3, 0..1);
            }
        }

        Ok(())
    }
}

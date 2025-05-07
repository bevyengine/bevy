use bevy_ecs::{prelude::World, query::QueryItem};
use bevy_render::{
    camera::ExtractedCamera,
    frame_graph::{render_pass_builder::RenderPassBuilder, FrameGraph},
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_phase::{TrackedRenderPass, ViewBinnedRenderPhases},
    render_resource::StoreOp,
    renderer::RenderDevice,
    view::{ExtractedView, ViewDepthTexture, ViewTarget},
};
use tracing::error;
#[cfg(feature = "trace")]
use tracing::info_span;

use super::{AlphaMask2d, Opaque2d};

/// A [`bevy_render::render_graph::Node`] that runs the
/// [`Opaque2d`] [`ViewBinnedRenderPhases`] and [`AlphaMask2d`] [`ViewBinnedRenderPhases`]
#[derive(Default)]
pub struct MainOpaquePass2dNode;
impl ViewNode for MainOpaquePass2dNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ExtractedView,
        &'static ViewTarget,
        &'static ViewDepthTexture,
    );

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        frame_graph: &mut FrameGraph,
        (camera, view, target, depth): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let (Some(opaque_phases), Some(alpha_mask_phases)) = (
            world.get_resource::<ViewBinnedRenderPhases<Opaque2d>>(),
            world.get_resource::<ViewBinnedRenderPhases<AlphaMask2d>>(),
        ) else {
            return Ok(());
        };

        let view_entity = graph.view_entity();
        let (Some(opaque_phase), Some(alpha_mask_phase)) = (
            opaque_phases.get(&view.retained_view_entity),
            alpha_mask_phases.get(&view.retained_view_entity),
        ) else {
            return Ok(());
        };

        let render_device = world.resource::<RenderDevice>();

        let mut pass_node_builder = frame_graph.create_pass_node_bulder("main_opaque_pass_2d");

        let color_attachment = target.get_color_attachment(&mut pass_node_builder)?;
        let depth_stencil_attachment =
            depth.get_depth_stencil_attachment(&mut pass_node_builder, StoreOp::Store)?;

        let mut builder = RenderPassBuilder::new(pass_node_builder);

        builder
            .add_color_attachment(color_attachment)
            .set_depth_stencil_attachment(depth_stencil_attachment)
            .set_camera_viewport(camera.viewport.clone());

        let mut tracked_render_pass = TrackedRenderPass::new(&render_device, builder);
        if !opaque_phase.is_empty() {
            #[cfg(feature = "trace")]
            let _opaque_main_pass_3d_span = info_span!("opaque_main_pass_2d").entered();
            if let Err(err) = opaque_phase.render(&mut tracked_render_pass, world, view_entity) {
                error!("Error encountered while rendering the 2d opaque phase {err:?}");
            }
        }

        // Alpha mask draws
        if !alpha_mask_phase.is_empty() {
            #[cfg(feature = "trace")]
            let _alpha_mask_main_pass_3d_span = info_span!("alpha_mask_main_pass_2d").entered();
            if let Err(err) = alpha_mask_phase.render(&mut tracked_render_pass, world, view_entity)
            {
                error!("Error encountered while rendering the 2d alpha mask phase {err:?}");
            }
        }

        Ok(())
    }
}

use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::{
    camera::ExtractedCamera,
    frame_graph::{FrameGraph, PassBuilder},
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_phase::{TrackedRenderPass, ViewSortedRenderPhases},
    render_resource::StoreOp,
    renderer::RenderDevice,
    view::{ExtractedView, ViewDepthTexture, ViewTarget},
};
use tracing::error;
#[cfg(feature = "trace")]
use tracing::info_span;

use super::Transparent3d;

/// A [`bevy_render::render_graph::Node`] that runs the [`Transparent3d`]
/// [`ViewSortedRenderPhases`].
#[derive(Default)]
pub struct MainTransparentPass3dNode;

impl ViewNode for MainTransparentPass3dNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ExtractedView,
        &'static ViewTarget,
        &'static ViewDepthTexture,
    );
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        frame_graph: &mut FrameGraph,
        (camera, view, target, depth): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.view_entity();
        let Some(transparent_phases) =
            world.get_resource::<ViewSortedRenderPhases<Transparent3d>>()
        else {
            return Ok(());
        };

        let Some(transparent_phase) = transparent_phases.get(&view.retained_view_entity) else {
            return Ok(());
        };

        if !transparent_phase.items.is_empty() {
            // Run the transparent pass, sorted back-to-front
            // NOTE: Scoped to drop the mutable borrow of render_context
            #[cfg(feature = "trace")]
            let _main_transparent_pass_3d_span = info_span!("main_transparent_pass_3d").entered();

            let render_device = world.resource::<RenderDevice>();

            let mut pass_builder =
                PassBuilder::new(frame_graph.create_pass_node_bulder("main_transparent_pass_3d"));

            let color_attachment = target.get_color_attachment(pass_builder.pass_node_builder())?;
            let depth_stencil_attachment = depth
                .get_depth_stencil_attachment(pass_builder.pass_node_builder(), StoreOp::Store)?;

            let mut builder = pass_builder.create_render_pass_builder();

            builder
                .set_pass_name("main_transparent_pass_3d")
                .add_color_attachment(color_attachment)
                .set_depth_stencil_attachment(depth_stencil_attachment)
                .set_camera_viewport(camera.viewport.clone());

            let mut tracked_render_pass = TrackedRenderPass::new(&render_device, builder);

            if let Err(err) = transparent_phase.render(&mut tracked_render_pass, world, view_entity)
            {
                error!("Error encountered while rendering the transparent phase {err:?}");
            }
        }

        // WebGL2 quirk: if ending with a render pass with a custom viewport, the viewport isn't
        // reset for the next render pass so add an empty render pass without a custom viewport
        #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
        if camera.viewport.is_some() {
            #[cfg(feature = "trace")]
            let _reset_viewport_pass_3d = info_span!("reset_viewport_pass_3d").entered();

            let mut pass_node_builder =
                frame_graph.create_pass_node_bulder("reset_viewport_pass_3d");
            let color_attachment = target.get_color_attachment(&mut pass_node_builder)?;
            let mut builder = RenderPassBuilder::new(pass_node_builder);
            Builder
                .set_pass_name("reset_viewport_pass_3d")
                .add_color_attachment(color_attachment);
        }

        Ok(())
    }
}

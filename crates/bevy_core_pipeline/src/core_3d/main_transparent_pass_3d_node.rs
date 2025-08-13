use crate::core_3d::Transparent3d;
use bevy_camera::{MainPassResolutionOverride, Viewport};
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::{
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_phase::ViewSortedRenderPhases,
    render_resource::{RenderPassDescriptor, StoreOp},
    renderer::RenderContext,
    view::{ExtractedView, ViewDepthTexture, ViewTarget},
};
use tracing::error;
#[cfg(feature = "trace")]
use tracing::info_span;

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
        Option<&'static MainPassResolutionOverride>,
    );
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (camera, view, target, depth, resolution_override): QueryItem<Self::ViewQuery>,
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

            let diagnostics = render_context.diagnostic_recorder();

            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("main_transparent_pass_3d"),
                color_attachments: &[Some(target.get_color_attachment())],
                // NOTE: For the transparent pass we load the depth buffer. There should be no
                // need to write to it, but store is set to `true` as a workaround for issue #3776,
                // https://github.com/bevyengine/bevy/issues/3776
                // so that wgpu does not clear the depth buffer.
                // As the opaque and alpha mask passes run first, opaque meshes can occlude
                // transparent ones.
                depth_stencil_attachment: Some(depth.get_attachment(StoreOp::Store)),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            let pass_span = diagnostics.pass_span(&mut render_pass, "main_transparent_pass_3d");

            if let Some(viewport) =
                Viewport::from_viewport_and_override(camera.viewport.as_ref(), resolution_override)
            {
                render_pass.set_camera_viewport(&viewport);
            }

            if let Err(err) = transparent_phase.render(&mut render_pass, world, view_entity) {
                error!("Error encountered while rendering the transparent phase {err:?}");
            }

            pass_span.end(&mut render_pass);
        }

        // WebGL2 quirk: if ending with a render pass with a custom viewport, the viewport isn't
        // reset for the next render pass so add an empty render pass without a custom viewport
        #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
        if camera.viewport.is_some() {
            #[cfg(feature = "trace")]
            let _reset_viewport_pass_3d = info_span!("reset_viewport_pass_3d").entered();
            let pass_descriptor = RenderPassDescriptor {
                label: Some("reset_viewport_pass_3d"),
                color_attachments: &[Some(target.get_color_attachment())],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            };

            render_context
                .command_encoder()
                .begin_render_pass(&pass_descriptor);
        }

        Ok(())
    }
}

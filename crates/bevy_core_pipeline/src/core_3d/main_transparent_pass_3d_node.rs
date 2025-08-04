use crate::core_3d::Transparent3d;
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::{
    camera::{ExtractedCamera, MainPassResolutionOverride},
    diagnostic::RecordDiagnostics,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_phase::{TrackedRenderPass, ViewSortedRenderPhases},
    render_resource::{CommandEncoderDescriptor, RenderPassDescriptor, StoreOp},
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
    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (camera, view, target, depth, resolution_override): QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
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
            let diagnostics = render_context.diagnostic_recorder();

            let color_attachments = [Some(target.get_color_attachment())];
            // NOTE: For the transparent pass we load the depth buffer. There should be no
            // need to write to it, but store is set to `true` as a workaround for issue #3776,
            // https://github.com/bevyengine/bevy/issues/3776
            // so that wgpu does not clear the depth buffer.
            // As the opaque and alpha mask passes run first, opaque meshes can occlude
            // transparent ones.
            let depth_stencil_attachment = Some(depth.get_attachment(StoreOp::Store));

            render_context.add_command_buffer_generation_task(move |render_device| {
                // Run the transparent pass, sorted back-to-front
                #[cfg(feature = "trace")]
                let _main_transparent_pass_3d_span =
                    info_span!("main_transparent_pass_3d").entered();

                // Command encoder setup
                let mut command_encoder =
                    render_device.create_command_encoder(&CommandEncoderDescriptor {
                        label: Some("main_transparent_pass_3d_command_encoder"),
                    });

                // Render pass setup
                let render_pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("main_transparent_pass_3d"),
                    color_attachments: &color_attachments,
                    depth_stencil_attachment,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                let mut render_pass = TrackedRenderPass::new(&render_device, render_pass);
                let pass_span = diagnostics.pass_span(&mut render_pass, "main_transparent_pass_3d");

                if let Some(viewport) = camera.viewport.as_ref() {
                    render_pass.set_camera_viewport(&viewport.with_override(resolution_override));
                }

                if let Err(err) = transparent_phase.render(&mut render_pass, world, view_entity) {
                    error!("Error encountered while rendering the transparent phase {err:?}");
                }

                pass_span.end(&mut render_pass);
                drop(render_pass);

                // WebGL2 quirk: if ending with a render pass with a custom viewport, the viewport isn't
                // reset for the next render pass so add an empty render pass without a custom viewport
                #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
                if camera.viewport.is_some() {
                    #[cfg(feature = "trace")]
                    let _reset_viewport_pass_3d = info_span!("reset_viewport_pass_3d").entered();
                    let pass_descriptor = RenderPassDescriptor {
                        label: Some("reset_viewport_pass_3d"),
                        color_attachments: &color_attachments,
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    };

                    command_encoder.begin_render_pass(&pass_descriptor);
                }

                command_encoder.finish()
            });
        }

        Ok(())
    }
}

use crate::core_2d::Opaque2d;
use bevy_ecs::{prelude::World, query::QueryItem};
use bevy_render::{
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_phase::{TrackedRenderPass, ViewSortedRenderPhases},
    render_resource::{CommandEncoderDescriptor, RenderPassDescriptor, StoreOp},
    renderer::RenderContext,
    view::{ViewDepthTexture, ViewTarget},
};
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;

/// A [`bevy_render::render_graph::Node`] that runs the [`Opaque2d`] [`SortedRenderPhase`]
#[derive(Default)]
pub struct MainOpaquePass2dNode;
impl ViewNode for MainOpaquePass2dNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static ViewDepthTexture,
    );

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (camera, target, depth): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let Some(opaque_phases) = world.get_resource::<ViewSortedRenderPhases<Opaque2d>>() else {
            return Ok(());
        };

        let diagnostics = render_context.diagnostic_recorder();

        let color_attachments = [Some(target.get_color_attachment())];
        let depth_stencil_attachment = Some(depth.get_attachment(StoreOp::Store));

        let view_entity = graph.view_entity();
        let Some(opaque_phase) = opaque_phases.get(&view_entity) else {
            return Ok(());
        };
        render_context.add_command_buffer_generation_task(move |render_device| {
            #[cfg(feature = "trace")]
            let _main_opaque_pass_2d_span = info_span!("main_opaque_pass_2d").entered();

            // Command encoder setup
            let mut command_encoder =
                render_device.create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("main_opaque_pass_2d_command_encoder"),
                });

            // Render pass setup
            let render_pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("main_opaque_pass_2d"),
                color_attachments: &color_attachments,
                depth_stencil_attachment,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            let mut render_pass = TrackedRenderPass::new(&render_device, render_pass);
            let pass_span = diagnostics.pass_span(&mut render_pass, "main_opaque_pass_2d");

            if let Some(viewport) = camera.viewport.as_ref() {
                render_pass.set_camera_viewport(viewport);
            }

            // Opaque draws
            if !opaque_phase.items.is_empty() {
                #[cfg(feature = "trace")]
                let _opaque_main_pass_2d_span = info_span!("opaque_main_pass_2d").entered();
                opaque_phase.render(&mut render_pass, world, view_entity);
            }

            pass_span.end(&mut render_pass);
            drop(render_pass);
            command_encoder.finish()
        });

        Ok(())
    }
}

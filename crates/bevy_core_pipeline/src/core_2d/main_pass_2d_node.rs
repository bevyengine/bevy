use crate::core_2d::Transparent2d;
use bevy_ecs::prelude::*;
use bevy_render::{
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
    render_graph::{Node, NodeRunError, RenderGraphContext},
    render_phase::RenderPhase,
    render_resource::RenderPassDescriptor,
    renderer::RenderContext,
    view::{ExtractedView, ViewTarget},
};
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;

pub struct MainPass2dNode {
    query: QueryState<
        (
            &'static ExtractedCamera,
            &'static RenderPhase<Transparent2d>,
            &'static ViewTarget,
        ),
        With<ExtractedView>,
    >,
}

impl FromWorld for MainPass2dNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: world.query_filtered(),
        }
    }
}

impl Node for MainPass2dNode {
    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.view_entity();
        let Ok((camera, transparent_phase, target)) = self.query.get_manual(world, view_entity)
        else {
            // no target
            return Ok(());
        };

        {
            #[cfg(feature = "trace")]
            let _main_pass_2d = info_span!("main_pass_2d").entered();

            let diagnostics = render_context.diagnostic_recorder();

            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("main_pass_2d"),
                color_attachments: &[Some(target.get_color_attachment())],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            let pass_span = diagnostics.pass_span(&mut render_pass, "main_pass_2d");

            if let Some(viewport) = camera.viewport.as_ref() {
                render_pass.set_camera_viewport(viewport);
            }

            transparent_phase.render(&mut render_pass, world, view_entity);

            pass_span.end(&mut render_pass);
        }

        // WebGL2 quirk: if ending with a render pass with a custom viewport, the viewport isn't
        // reset for the next render pass so add an empty render pass without a custom viewport
        #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
        if camera.viewport.is_some() {
            #[cfg(feature = "trace")]
            let _reset_viewport_pass_2d = info_span!("reset_viewport_pass_2d").entered();
            let pass_descriptor = RenderPassDescriptor {
                label: Some("reset_viewport_pass_2d"),
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

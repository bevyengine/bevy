use crate::core_3d::Transparent3d;
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::{
    camera::ExtractedCamera,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_phase::{RenderPhase, TrackedRenderPass},
    render_resource::{
        CommandEncoderDescriptor, LoadOp, Operations, RenderPassDepthStencilAttachment,
        RenderPassDescriptor,
    },
    renderer::{RenderContext, RenderDevice},
    view::{ViewDepthTexture, ViewTarget},
};
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;

/// A [`bevy_render::render_graph::Node`] that runs the [`Transparent3d`] [`RenderPhase`].
#[derive(Default)]
pub struct MainTransparentPass3dNode;

impl ViewNode for MainTransparentPass3dNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static RenderPhase<Transparent3d>,
        &'static ViewTarget,
        &'static ViewDepthTexture,
    );
    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (camera, transparent_phase, target, depth): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.view_entity();

        let color_attachment = target.get_color_attachment(Operations {
            load: LoadOp::Load,
            store: true,
        });

        let transparent_phase_render_task = move |render_device: RenderDevice| {
            // Run the transparent pass, sorted back-to-front
            #[cfg(feature = "trace")]
            let _main_transparent_pass_3d_span = info_span!("main_transparent_pass_3d").entered();

            let mut command_encoder =
                render_device.create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("main_transparent_pass_3d_command_encoder"),
                });

            let render_pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("main_transparent_pass_3d"),
                // NOTE: The transparent pass loads the color buffer as well as overwriting it where appropriate.
                color_attachments: &[Some(color_attachment)],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &depth.view,
                    // NOTE: For the transparent pass we load the depth buffer. There should be no
                    // need to write to it, but store is set to `true` as a workaround for issue #3776,
                    // https://github.com/bevyengine/bevy/issues/3776
                    // so that wgpu does not clear the depth buffer.
                    // As the opaque and alpha mask passes run first, opaque meshes can occlude
                    // transparent ones.
                    depth_ops: Some(Operations {
                        load: LoadOp::Load,
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            let mut render_pass = TrackedRenderPass::new(&render_device, render_pass);

            if let Some(viewport) = camera.viewport.as_ref() {
                render_pass.set_camera_viewport(viewport);
            }

            transparent_phase.render(&mut render_pass, world, view_entity);

            drop(render_pass);
            command_encoder.finish()
        };

        if !transparent_phase.items.is_empty() {
            render_context.add_command_buffer_generation_task(transparent_phase_render_task);
        }

        // WebGL2 quirk: if ending with a render pass with a custom viewport, the viewport isn't
        // reset for the next render pass so add an empty render pass without a custom viewport
        #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
        if camera.viewport.is_some() {
            #[cfg(feature = "trace")]
            let _reset_viewport_pass_3d = info_span!("reset_viewport_pass_3d").entered();
            let pass_descriptor = RenderPassDescriptor {
                label: Some("reset_viewport_pass_3d"),
                color_attachments: &[Some(target.get_color_attachment(Operations {
                    load: LoadOp::Load,
                    store: true,
                }))],
                depth_stencil_attachment: None,
            };

            render_context
                .command_encoder()
                .begin_render_pass(&pass_descriptor);
        }

        Ok(())
    }
}

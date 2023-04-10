use super::ViewTransmissionTexture;
use crate::core_3d::Transmissive3d;
use bevy_ecs::prelude::*;
use bevy_render::{
    camera::ExtractedCamera,
    render_graph::{Node, NodeRunError, RenderGraphContext},
    render_phase::RenderPhase,
    render_resource::{
        Extent3d, LoadOp, Operations, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    },
    renderer::RenderContext,
    view::{ExtractedView, ViewDepthTexture, ViewTarget},
};
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;

/// A [`Node`] that runs the [`Transmissive3d`] [`RenderPhase`].
pub struct MainTransmissivePass3dNode {
    query: QueryState<
        (
            &'static ExtractedCamera,
            &'static RenderPhase<Transmissive3d>,
            &'static ViewTarget,
            &'static ViewTransmissionTexture,
            &'static ViewDepthTexture,
        ),
        With<ExtractedView>,
    >,
}

impl FromWorld for MainTransmissivePass3dNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: world.query_filtered(),
        }
    }
}

impl Node for MainTransmissivePass3dNode {
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
        let Ok((
            camera,
            transmissive_phase,
            target,
            transmission,
            depth,
        )) = self.query.get_manual(world, view_entity) else {
            // No window
            return Ok(());
        };

        let physical_target_size = camera.physical_target_size.unwrap();
        render_context.command_encoder().copy_texture_to_texture(
            target.main_texture().as_image_copy(),
            transmission.texture.as_image_copy(),
            Extent3d {
                width: physical_target_size.x,
                height: physical_target_size.y,
                depth_or_array_layers: 1,
            },
        );

        if !transmissive_phase.items.is_empty() {
            // Run the transmissive pass, sorted back-to-front
            // NOTE: Scoped to drop the mutable borrow of render_context
            #[cfg(feature = "trace")]
            let _main_transmissive_pass_3d_span = info_span!("main_transmissive_pass_3d").entered();

            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("main_transmissive_pass_3d"),
                // NOTE: The transmissive pass loads the color buffer as well as overwriting it where appropriate.
                color_attachments: &[Some(target.get_color_attachment(Operations {
                    load: LoadOp::Load,
                    store: true,
                }))],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &depth.view,
                    // NOTE: The transmissive main pass loads the depth buffer and possibly overwrites it
                    depth_ops: Some(Operations {
                        load: LoadOp::Load,
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            if let Some(viewport) = camera.viewport.as_ref() {
                render_pass.set_camera_viewport(viewport);
            }

            transmissive_phase.render(&mut render_pass, world, view_entity);
        }

        Ok(())
    }
}

use crate::{
    clear_color::{ClearColor, ClearColorConfig},
    core_3d::{Camera3d, Opaque3d},
    prepass::{DepthPrepass, MotionVectorPrepass, NormalPrepass},
};
use bevy_ecs::prelude::*;
use bevy_render::{
    camera::ExtractedCamera,
    render_graph::{Node, NodeRunError, RenderGraphContext},
    render_phase::RenderPhase,
    render_resource::{LoadOp, Operations, RenderPassDepthStencilAttachment, RenderPassDescriptor},
    renderer::RenderContext,
    view::{ExtractedView, ViewDepthTexture, ViewTarget},
};
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;

use super::{AlphaMask3d, Camera3dDepthLoadOp};

/// A [`Node`] that runs the [`Opaque3d`] and [`AlphaMask3d`] [`RenderPhase`].
pub struct MainOpaquePass3dNode {
    query: QueryState<
        (
            &'static ExtractedCamera,
            &'static RenderPhase<Opaque3d>,
            &'static RenderPhase<AlphaMask3d>,
            &'static Camera3d,
            &'static ViewTarget,
            &'static ViewDepthTexture,
            Option<&'static DepthPrepass>,
            Option<&'static NormalPrepass>,
            Option<&'static MotionVectorPrepass>,
        ),
        With<ExtractedView>,
    >,
}

impl MainOpaquePass3dNode {
    pub fn new(world: &mut World) -> Self {
        Self {
            query: world.query_filtered(),
        }
    }
}

impl Node for MainOpaquePass3dNode {
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
            opaque_phase,
            alpha_mask_phase,
            camera_3d,
            target,
            depth,
            depth_prepass,
            normal_prepass,
            motion_vector_prepass
        )) = self.query.get_manual(world, view_entity) else {
            // No window
            return Ok(());
        };

        // Run the opaque pass, sorted front-to-back
        // NOTE: Scoped to drop the mutable borrow of render_context
        #[cfg(feature = "trace")]
        let _main_opaque_pass_3d_span = info_span!("main_opaque_pass_3d").entered();

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("main_opaque_pass_3d"),
            // NOTE: The opaque pass loads the color
            // buffer as well as writing to it.
            color_attachments: &[Some(target.get_color_attachment(Operations {
                load: match camera_3d.clear_color {
                    ClearColorConfig::Default => {
                        LoadOp::Clear(world.resource::<ClearColor>().0.into())
                    }
                    ClearColorConfig::Custom(color) => LoadOp::Clear(color.into()),
                    ClearColorConfig::None => LoadOp::Load,
                },
                store: true,
            }))],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &depth.view,
                // NOTE: The opaque main pass loads the depth buffer and possibly overwrites it
                depth_ops: Some(Operations {
                    load: if depth_prepass.is_some()
                        || normal_prepass.is_some()
                        || motion_vector_prepass.is_some()
                    {
                        // if any prepass runs, it will generate a depth buffer so we should use it,
                        // even if only the normal_prepass is used.
                        Camera3dDepthLoadOp::Load
                    } else {
                        // NOTE: 0.0 is the far plane due to bevy's use of reverse-z projections.
                        camera_3d.depth_load_op.clone()
                    }
                    .into(),
                    store: true,
                }),
                stencil_ops: None,
            }),
        });

        if let Some(viewport) = camera.viewport.as_ref() {
            render_pass.set_camera_viewport(viewport);
        }

        opaque_phase.render(&mut render_pass, world, view_entity);

        if !alpha_mask_phase.items.is_empty() {
            alpha_mask_phase.render(&mut render_pass, world, view_entity);
        }

        Ok(())
    }
}

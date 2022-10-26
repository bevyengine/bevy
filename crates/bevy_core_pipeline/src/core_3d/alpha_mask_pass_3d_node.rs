use crate::core_3d::AlphaMask3d;

use bevy_ecs::prelude::*;

use bevy_render::{
    camera::ExtractedCamera,
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotInfo, SlotType},
    render_phase::{DrawFunctions, RenderPhase, TrackedRenderPass},
    render_resource::{LoadOp, Operations, RenderPassDepthStencilAttachment, RenderPassDescriptor},
    renderer::RenderContext,
    view::{ExtractedView, ViewDepthTexture, ViewTarget},
};

#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;

pub struct AlphaMaskPass3dNode {
    query: QueryState<
        (
            &'static ExtractedCamera,
            &'static RenderPhase<AlphaMask3d>,
            &'static ViewTarget,
            &'static ViewDepthTexture,
        ),
        With<ExtractedView>,
    >,
}

impl AlphaMaskPass3dNode {
    pub const IN_VIEW: &'static str = "view";

    pub fn new(world: &mut World) -> Self {
        Self {
            query: world.query_filtered(),
        }
    }
}

impl Node for AlphaMaskPass3dNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(
            AlphaMaskPass3dNode::IN_VIEW,
            SlotType::Entity,
        )]
    }

    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
        let (camera, alpha_mask_phase, target, depth) =
            match self.query.get_manual(world, view_entity) {
                Ok(query) => query,
                Err(_) => {
                    return Ok(());
                } // No window
            };

        if !alpha_mask_phase.items.is_empty() {
            // Run the alpha mask pass, sorted front-to-back
            #[cfg(feature = "trace")]
            let _main_alpha_mask_pass_3d_span = info_span!("main_alpha_mask_pass_3d").entered();
            let pass_descriptor = RenderPassDescriptor {
                label: Some("main_alpha_mask_pass_3d"),
                // NOTE: The alpha_mask pass loads the color buffer as well as overwriting it where appropriate.
                color_attachments: &[Some(target.get_color_attachment(Operations {
                    load: LoadOp::Load,
                    store: true,
                }))],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &depth.view,
                    // NOTE: The alpha mask pass loads the depth buffer and possibly overwrites it
                    depth_ops: Some(Operations {
                        load: LoadOp::Load,
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            };

            let draw_functions = world.resource::<DrawFunctions<AlphaMask3d>>();

            let render_pass = render_context
                .command_encoder
                .begin_render_pass(&pass_descriptor);
            let mut draw_functions = draw_functions.write();
            let mut tracked_pass = TrackedRenderPass::new(render_pass);
            if let Some(viewport) = camera.viewport.as_ref() {
                tracked_pass.set_camera_viewport(viewport);
            }
            for item in &alpha_mask_phase.items {
                let draw_function = draw_functions.get_mut(item.draw_function).unwrap();
                draw_function.draw(world, &mut tracked_pass, view_entity, item);
            }
        }

        Ok(())
    }
}

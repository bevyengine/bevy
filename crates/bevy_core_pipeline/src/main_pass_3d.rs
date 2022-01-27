use crate::{AlphaMask3d, Opaque3d, Transparent3d};
use bevy_ecs::prelude::*;
use bevy_render::{
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotInfo, SlotType},
    render_phase::{DrawFunctions, RenderPhase, TrackedRenderPass},
    render_resource::{LoadOp, Operations, RenderPassDepthStencilAttachment, RenderPassDescriptor},
    renderer::RenderContext,
    view::{ExtractedView, ViewDepthTexture, ViewTarget},
};

pub struct MainPass3dNode {
    query: QueryState<
        (
            &'static RenderPhase<Opaque3d>,
            &'static RenderPhase<AlphaMask3d>,
            &'static RenderPhase<Transparent3d>,
            &'static ViewTarget,
            &'static ViewDepthTexture,
        ),
        With<ExtractedView>,
    >,
}

impl MainPass3dNode {
    pub const IN_VIEW: &'static str = "view";

    pub fn new(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

impl Node for MainPass3dNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(MainPass3dNode::IN_VIEW, SlotType::Entity)]
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
        let (opaque_phase, alpha_mask_phase, transparent_phase, target, depth) =
            match self.query.get_manual(world, view_entity) {
                Ok(query) => query,
                Err(_) => return Ok(()), // No window
            };

        {
            // Run the opaque pass, sorted front-to-back
            // NOTE: Scoped to drop the mutable borrow of render_context
            let pass_descriptor = RenderPassDescriptor {
                label: Some("main_opaque_pass_3d"),
                // NOTE: The opaque pass loads the color
                // buffer as well as writing to it.
                color_attachments: &[target.get_color_attachment(Operations {
                    load: LoadOp::Load,
                    store: true,
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &depth.view,
                    // NOTE: The opaque main pass loads the depth buffer and possibly overwrites it
                    depth_ops: Some(Operations {
                        load: LoadOp::Load,
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            };

            let draw_functions = world.get_resource::<DrawFunctions<Opaque3d>>().unwrap();

            let render_pass = render_context
                .command_encoder
                .begin_render_pass(&pass_descriptor);
            let mut draw_functions = draw_functions.write();
            let mut tracked_pass = TrackedRenderPass::new(render_pass);
            for item in opaque_phase.items.iter() {
                let draw_function = draw_functions.get_mut(item.draw_function).unwrap();
                draw_function.draw(world, &mut tracked_pass, view_entity, item);
            }
        }

        {
            // Run the alpha mask pass, sorted front-to-back
            // NOTE: Scoped to drop the mutable borrow of render_context
            let pass_descriptor = RenderPassDescriptor {
                label: Some("main_alpha_mask_pass_3d"),
                // NOTE: The alpha_mask pass loads the color buffer as well as overwriting it where appropriate.
                color_attachments: &[target.get_color_attachment(Operations {
                    load: LoadOp::Load,
                    store: true,
                })],
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

            let draw_functions = world.get_resource::<DrawFunctions<AlphaMask3d>>().unwrap();

            let render_pass = render_context
                .command_encoder
                .begin_render_pass(&pass_descriptor);
            let mut draw_functions = draw_functions.write();
            let mut tracked_pass = TrackedRenderPass::new(render_pass);
            for item in alpha_mask_phase.items.iter() {
                let draw_function = draw_functions.get_mut(item.draw_function).unwrap();
                draw_function.draw(world, &mut tracked_pass, view_entity, item);
            }
        }

        {
            // Run the transparent pass, sorted back-to-front
            // NOTE: Scoped to drop the mutable borrow of render_context
            let pass_descriptor = RenderPassDescriptor {
                label: Some("main_transparent_pass_3d"),
                // NOTE: The transparent pass loads the color buffer as well as overwriting it where appropriate.
                color_attachments: &[target.get_color_attachment(Operations {
                    load: LoadOp::Load,
                    store: true,
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &depth.view,
                    // NOTE: There is a useful optimization to set `store: false` to prevent wasting bandwidth writing to the
                    // depth buffer. This only works if the `main_transparent_pass_3d` render pass is the last one executed (because
                    // no subsequent pass will need to use the depth buffer).
                    // If it is not the last executed render pass then WGPU will re-initialize the depth buffer before the
                    // next pass. This is affects users who want to perform post-processing using the depth buffer.
                    //
                    // Some time in the future it may be possible to inspect if we will be the last render pass and apply
                    // this optimization, but for now this must always be `true`.
                    // Some rendering linters may complain about this in the default case.
                    depth_ops: Some(Operations {
                        load: LoadOp::Load,
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            };

            let draw_functions = world
                .get_resource::<DrawFunctions<Transparent3d>>()
                .unwrap();

            let render_pass = render_context
                .command_encoder
                .begin_render_pass(&pass_descriptor);
            let mut draw_functions = draw_functions.write();
            let mut tracked_pass = TrackedRenderPass::new(render_pass);
            for item in transparent_phase.items.iter() {
                let draw_function = draw_functions.get_mut(item.draw_function).unwrap();
                draw_function.draw(world, &mut tracked_pass, view_entity, item);
            }
        }

        Ok(())
    }
}

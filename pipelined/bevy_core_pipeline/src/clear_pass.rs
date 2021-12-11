use crate::ClearColor;
use bevy_ecs::prelude::*;
use bevy_render2::{
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotInfo},
    render_resource::{LoadOp, Operations, RenderPassDepthStencilAttachment, RenderPassDescriptor},
    renderer::RenderContext,
    view::{ExtractedView, ViewDepthTexture, ViewTarget},
};

pub struct ClearPassNode {
    query:
        QueryState<(&'static ViewTarget, Option<&'static ViewDepthTexture>), With<ExtractedView>>,
}

impl ClearPassNode {
    pub fn new(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

impl Node for ClearPassNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![]
    }

    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        /* This gets all ViewTargets and ViewDepthTextures and clears its attachments */
        for (target, depth) in self.query.iter_manual(world) {
            let clear_color = world.get_resource::<ClearColor>().unwrap();
            let pass_descriptor = RenderPassDescriptor {
                label: Some("clear_pass"),
                color_attachments: &[target.get_color_attachment(Operations {
                    load: LoadOp::Clear(clear_color.0.into()),
                    store: true,
                })],
                depth_stencil_attachment: depth.map(|depth| RenderPassDepthStencilAttachment {
                    view: &depth.view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(0.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            };

            render_context
                .command_encoder
                .begin_render_pass(&pass_descriptor);
        }

        Ok(())
    }
}

use crate::{ClearColor, Transparent3d};
use bevy_ecs::prelude::*;
use bevy_render2::{
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotInfo, SlotType},
    render_phase::{DrawFunctions, RenderPhase, TrackedRenderPass},
    render_resource::{
        LoadOp, Operations, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
        RenderPassDescriptor,
    },
    renderer::RenderContext,
    view::{ExtractedView, ViewDepthTexture, ViewTarget},
};

pub struct MainPass3dNode {
    query: QueryState<
        (
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
        let (transparent_phase, target, depth) = self
            .query
            .get_manual(world, view_entity)
            .expect("view entity should exist");
        let clear_color = world.get_resource::<ClearColor>().unwrap();
        let pass_descriptor = RenderPassDescriptor {
            label: Some("main_pass_3d"),
            color_attachments: &[RenderPassColorAttachment {
                view: if let Some(sampled_target) = &target.sampled_target {
                    sampled_target
                } else {
                    &target.view
                },
                resolve_target: if target.sampled_target.is_some() {
                    Some(&target.view)
                } else {
                    None
                },
                ops: Operations {
                    load: LoadOp::Clear(clear_color.0.into()),
                    store: true,
                },
            }],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &depth.view,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(0.0),
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
        Ok(())
    }
}

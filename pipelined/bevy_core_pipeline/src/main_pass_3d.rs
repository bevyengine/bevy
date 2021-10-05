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
    view::ExtractedView,
};

pub struct MainPass3dNode {
    query: QueryState<&'static RenderPhase<Transparent3d>, With<ExtractedView>>,
}

impl MainPass3dNode {
    pub const IN_COLOR_ATTACHMENT: &'static str = "color_attachment";
    pub const IN_DEPTH: &'static str = "depth";
    pub const IN_VIEW: &'static str = "view";

    pub fn new(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

impl Node for MainPass3dNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![
            SlotInfo::new(MainPass3dNode::IN_COLOR_ATTACHMENT, SlotType::TextureView),
            SlotInfo::new(MainPass3dNode::IN_DEPTH, SlotType::TextureView),
            SlotInfo::new(MainPass3dNode::IN_VIEW, SlotType::Entity),
        ]
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
        let color_attachment_texture = graph.get_input_texture(Self::IN_COLOR_ATTACHMENT)?;
        let clear_color = world.get_resource::<ClearColor>().unwrap();
        let depth_texture = graph.get_input_texture(Self::IN_DEPTH)?;
        let pass_descriptor = RenderPassDescriptor {
            label: Some("main_pass_3d"),
            color_attachments: &[RenderPassColorAttachment {
                view: color_attachment_texture,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(clear_color.0.into()),
                    store: true,
                },
            }],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: depth_texture,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(0.0),
                    store: true,
                }),
                stencil_ops: None,
            }),
        };

        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
        let draw_functions = world
            .get_resource::<DrawFunctions<Transparent3d>>()
            .unwrap();
        let transparent_phase = self
            .query
            .get_manual(world, view_entity)
            .expect("view entity should exist");

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

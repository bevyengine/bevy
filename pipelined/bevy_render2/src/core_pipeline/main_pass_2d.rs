use crate::{
    color::Color,
    core_pipeline::Transparent2dPhase,
    pass::{
        LoadOp, Operations, PassDescriptor, RenderPass, RenderPassColorAttachment,
        TextureAttachment,
    },
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotInfo, SlotType},
    render_phase::{DrawFunctions, RenderPhase, TrackedRenderPass},
    renderer::RenderContext,
    view::ExtractedView,
};
use bevy_ecs::prelude::*;

pub struct MainPass2dNode {
    query: QueryState<&'static RenderPhase<Transparent2dPhase>, With<ExtractedView>>,
}

impl MainPass2dNode {
    pub const IN_COLOR_ATTACHMENT: &'static str = "color_attachment";
    pub const IN_VIEW: &'static str = "view";

    pub fn new(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

impl Node for MainPass2dNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![
            SlotInfo::new(MainPass2dNode::IN_COLOR_ATTACHMENT, SlotType::TextureView),
            SlotInfo::new(MainPass2dNode::IN_VIEW, SlotType::Entity),
        ]
    }

    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut dyn RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let color_attachment_texture = graph.get_input_texture(Self::IN_COLOR_ATTACHMENT)?;
        let pass_descriptor = PassDescriptor {
            color_attachments: vec![RenderPassColorAttachment {
                attachment: TextureAttachment::Id(color_attachment_texture),
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::rgb(0.4, 0.4, 0.4)),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
            sample_count: 1,
        };

        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
        let draw_functions = world.get_resource::<DrawFunctions>().unwrap();

        let transparent_phase = self
            .query
            .get_manual(world, view_entity)
            .expect("view entity should exist");

        render_context.begin_render_pass(
            &pass_descriptor,
            &mut |render_pass: &mut dyn RenderPass| {
                let mut draw_functions = draw_functions.write();
                let mut tracked_pass = TrackedRenderPass::new(render_pass);
                for drawable in transparent_phase.drawn_things.iter() {
                    let draw_function = draw_functions.get_mut(drawable.draw_function).unwrap();
                    draw_function.draw(
                        world,
                        &mut tracked_pass,
                        view_entity,
                        drawable.draw_key,
                        drawable.sort_key,
                    );
                }
            },
        );
        Ok(())
    }
}

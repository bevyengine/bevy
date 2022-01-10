use crate::Transparent2d;
use bevy_ecs::prelude::*;
use bevy_math::Vec2;
use bevy_render::{
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotInfo, SlotType},
    render_phase::{DrawFunctions, RenderPhase, TrackedRenderPass},
    render_resource::{LoadOp, Operations, RenderPassDescriptor},
    renderer::RenderContext,
    view::{ExtractedView, ViewTarget},
};

pub struct MainPass2dNode {
    query: QueryState<(
        &'static RenderPhase<Transparent2d>,
        &'static ViewTarget,
        &'static ExtractedView,
    )>,
}

impl MainPass2dNode {
    pub const IN_VIEW: &'static str = "view";

    pub fn new(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

impl Node for MainPass2dNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(MainPass2dNode::IN_VIEW, SlotType::Entity)]
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
        let (transparent_phase, target, view) = self
            .query
            .get_manual(world, view_entity)
            .expect("view entity should exist");

        let pass_descriptor = RenderPassDescriptor {
            label: Some("main_pass_2d"),
            color_attachments: &[target.get_color_attachment(Operations {
                load: LoadOp::Load,
                store: true,
            })],
            depth_stencil_attachment: None,
        };

        let draw_functions = world
            .get_resource::<DrawFunctions<Transparent2d>>()
            .unwrap();

        let mut render_pass = render_context
            .command_encoder
            .begin_render_pass(&pass_descriptor);

        if let Some(viewport) = &view.viewport {
            let target_size = Vec2::new(view.width as f32, view.height as f32);
            let pos = viewport.scaled_pos(target_size);
            let size = viewport.scaled_size(target_size);
            render_pass.set_viewport(
                pos.x,
                pos.y,
                size.x,
                size.y,
                viewport.min_depth,
                viewport.max_depth,
            );
        }

        let mut draw_functions = draw_functions.write();
        let mut tracked_pass = TrackedRenderPass::new(render_pass);
        for item in &transparent_phase.items {
            let draw_function = draw_functions.get_mut(item.draw_function).unwrap();
            draw_function.draw(world, &mut tracked_pass, view_entity, item);
        }
        Ok(())
    }
}

use bevy_ecs::prelude::*;
use bevy_ecs::query::QueryState;
use bevy_render::{
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotInfo, SlotType},
    render_resource::{
        LoadOp, Operations, RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineCache,
    },
    renderer::RenderContext,
    view::{ExtractedView, ViewTarget},
};

use super::UpscalingTarget;

pub struct UpscalingNode {
    query: QueryState<(&'static ViewTarget, &'static UpscalingTarget), With<ExtractedView>>,
}

impl UpscalingNode {
    pub const IN_VIEW: &'static str = "view";

    pub fn new(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

impl Node for UpscalingNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(UpscalingNode::IN_VIEW, SlotType::Entity)]
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

        let render_pipeline_cache = world.get_resource::<RenderPipelineCache>().unwrap();

        let (target, upscaling_target) = match self.query.get_manual(world, view_entity) {
            Ok(query) => query,
            Err(_) => return Ok(()),
        };

        let pipeline = match render_pipeline_cache.get(upscaling_target.pipeline) {
            Some(pipeline) => pipeline,
            None => return Ok(()),
        };

        let pass_descriptor = RenderPassDescriptor {
            label: Some("upscaling_pass"),
            color_attachments: &[RenderPassColorAttachment {
                view: &target.out_texture,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Default::default()), // TODO dont_care
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        };

        let mut render_pass = render_context
            .command_encoder
            .begin_render_pass(&pass_descriptor);

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, &upscaling_target.ldr_texture_bind_group, &[]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}

use bevy_ecs::{
    query::{QueryState, With},
    world::World,
};

use bevy_render::{
    camera::ExtractedCamera,
    picking::{copy_to_buffer, Picking, PickingTextures},
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotInfo, SlotType},
    renderer::RenderContext,
    view::ViewDepthTexture,
};
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;

pub struct PickingNode {
    query: QueryState<
        (
            &'static Picking,
            &'static PickingTextures,
            Option<&'static ViewDepthTexture>,
        ),
        With<ExtractedCamera>,
    >,
}

impl PickingNode {
    pub const IN_VIEW: &'static str = "view";

    pub fn new(world: &mut World) -> Self {
        Self {
            query: world.query_filtered(),
        }
    }
}

impl Node for PickingNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(PickingNode::IN_VIEW, SlotType::Entity)]
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
        let (picking, picking_textures, depth) =
            if let Ok(result) = self.query.get_manual(world, view_entity) {
                result
            } else {
                // no target
                return Ok(());
            };
        {
            #[cfg(feature = "trace")]
            let _picking_pass = info_span!("picking_pass").entered();

            copy_to_buffer(picking, picking_textures, depth, render_context);
        }

        Ok(())
    }
}

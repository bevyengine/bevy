use crate::core_2d::{camera_2d::Camera2d, Transparent2d};
use bevy_ecs::prelude::*;
use bevy_render::{
    camera::ExtractedCamera,
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotInfo, SlotType},
    render_phase::RenderPhase,
    renderer::RenderContext,
    view::{ExtractedView, ViewTarget},
};
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;

pub struct MainPass2dNode {
    query: QueryState<
        (
            &'static ExtractedCamera,
            &'static RenderPhase<Transparent2d>,
            &'static ViewTarget,
            &'static Camera2d,
        ),
        With<ExtractedView>,
    >,
}

impl MainPass2dNode {
    pub const IN_VIEW: &'static str = "view";

    pub fn new(world: &mut World) -> Self {
        Self {
            query: world.query_filtered(),
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
        let (camera, transparent_phase, target, camera_2d) =
            if let Ok(result) = self.query.get_manual(world, view_entity) {
                result
            } else {
                // no target
                return Ok(());
            };
        {
            #[cfg(feature = "trace")]
            let _main_pass_2d = info_span!("main_pass_2d").entered();

            render_context
                .render_pass(view_entity)
                .set_label("main_pass_2d")
                .add_view_target(target)
                .set_color_ops(camera_2d.clear_color.load_op(world), true)
                .begin()
                .set_camera_viewport(camera)
                .render_phase(transparent_phase, world);
        }

        // WebGL2 quirk: if ending with a render pass with a custom viewport, the viewport isn't
        // reset for the next render pass so add an empty render pass without a custom viewport
        #[cfg(feature = "webgl")]
        if camera.viewport.is_some() {
            use bevy_render::render_resource::LoadOp;

            #[cfg(feature = "trace")]
            let _reset_viewport_pass_2d = info_span!("reset_viewport_pass_2d").entered();

            render_context
                .render_pass(view_entity)
                .set_label("reset_view_port_pass_2d")
                .add_view_target(target)
                .set_color_ops(LoadOp::Load, true)
                .begin();
        }

        Ok(())
    }
}

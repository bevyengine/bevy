use crate::{
    clear_color::{ClearColor, ClearColorConfig},
    core_2d::{camera_2d::Camera2d, Transparent2d},
};
use bevy_ecs::prelude::*;
use bevy_render::render_phase::TrackedRenderPass;
use bevy_render::{
    camera::ExtractedCamera,
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotInfo, SlotType},
    render_phase::RenderPhase,
    render_resource::{LoadOp, Operations},
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
            let color_ops = Operations {
                load: match camera_2d.clear_color {
                    ClearColorConfig::Default => {
                        LoadOp::Clear(world.resource::<ClearColor>().0.into())
                    }
                    ClearColorConfig::Custom(color) => LoadOp::Clear(color.into()),
                    ClearColorConfig::None => LoadOp::Load,
                },
                store: true,
            };

            let mut render_pass = TrackedRenderPass::create_for_camera(
                render_context,
                "main_pass_2d",
                view_entity,
                target,
                color_ops,
                None,
                None,
                &camera.viewport,
            );

            render_pass.render_phase(transparent_phase, world);
        }

        // WebGL2 quirk: if ending with a render pass with a custom viewport, the viewport isn't
        // reset for the next render pass so add an empty render pass without a custom viewport
        #[cfg(feature = "webgl")]
        if camera.viewport.is_some() {
            let _render_pass = TrackedRenderPass::create_for_camera(
                render_context,
                "reset_viewport_pass_2d",
                view_entity,
                target,
                Operations {
                    load: LoadOp::Load,
                    store: true,
                },
                None,
                None,
                &None,
            );
        }

        Ok(())
    }
}

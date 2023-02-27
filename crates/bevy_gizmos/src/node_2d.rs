use bevy_ecs::prelude::*;
use bevy_render::{
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotInfo, SlotType},
    render_phase::RenderPhase,
    render_resource::{LoadOp, Operations, RenderPassDescriptor},
    renderer::RenderContext,
    view::{ExtractedView, ViewTarget},
};

use crate::pipeline_2d::GizmoLine2d;

pub struct GizmoNode2d {
    view_query:
        QueryState<(&'static ViewTarget, &'static RenderPhase<GizmoLine2d>), With<ExtractedView>>,
}

impl GizmoNode2d {
    pub const IN_VIEW: &'static str = "view";
    pub const NAME: &'static str = "gizmo_node_2d";

    pub fn new(world: &mut World) -> Self {
        Self {
            view_query: QueryState::new(world),
        }
    }
}

impl Node for GizmoNode2d {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(Self::IN_VIEW, SlotType::Entity)]
    }

    fn update(&mut self, world: &mut World) {
        self.view_query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
        let Ok((
            view_target,
            gizmo_phase,
        )) = self.view_query.get_manual(world, view_entity) else {
            return Ok(());
        };
        {
            #[cfg(feature = "trace")]
            let _gizmo_line_2d_pass = info_span!("gizmo_line_2d_pass").entered();

            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("gizmo_line_2d"),
                color_attachments: &[Some(view_target.get_color_attachment(Operations {
                    load: LoadOp::Load,
                    store: true,
                }))],
                depth_stencil_attachment: None,
            });

            gizmo_phase.render(&mut render_pass, world, view_entity);
        }
        Ok(())
    }
}

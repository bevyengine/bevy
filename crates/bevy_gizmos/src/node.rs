use bevy_core_pipeline::core_3d::Camera3dDepthLoadOp;
use bevy_ecs::prelude::*;
use bevy_render::{
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotInfo, SlotType},
    render_phase::RenderPhase,
    render_resource::{LoadOp, Operations, RenderPassDepthStencilAttachment, RenderPassDescriptor},
    renderer::RenderContext,
    view::{ExtractedView, ViewDepthTexture, ViewTarget},
};

use crate::pipeline_3d::GizmoLine3d;

pub struct GizmoNode {
    view_query: QueryState<
        (
            &'static ViewTarget,
            &'static RenderPhase<GizmoLine3d>,
            &'static ViewDepthTexture,
        ),
        With<ExtractedView>,
    >,
}

impl GizmoNode {
    pub const IN_VIEW: &'static str = "view";
    pub const NAME: &'static str = "gizmo_node";

    pub fn new(world: &mut World) -> Self {
        Self {
            view_query: QueryState::new(world),
        }
    }
}

impl Node for GizmoNode {
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
            depth,
        )) = self.view_query.get_manual(world, view_entity) else {
            return Ok(());
        };

        {
            #[cfg(feature = "trace")]
            let _main_opaque_pass_3d_span = info_span!("gizmo_line_3d_pass").entered();

            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("gizmo_line_3d"),
                color_attachments: &[Some(view_target.get_color_attachment(Operations {
                    load: LoadOp::Load,
                    store: true,
                }))],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &depth.view,
                    depth_ops: Some(Operations {
                        load: Camera3dDepthLoadOp::Load.into(),
                        store: false,
                    }),
                    stencil_ops: None,
                }),
            });

            gizmo_phase.render(&mut render_pass, world, view_entity);
        }
        Ok(())
    }
}

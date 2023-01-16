use crate::{
    core_3d::{AlphaMask3d, Camera3d, Opaque3d, Transparent3d},
    prepass::{DepthPrepass, NormalPrepass},
};
use bevy_ecs::prelude::*;
use bevy_render::{
    camera::ExtractedCamera,
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotInfo, SlotType},
    render_phase::RenderPhase,
    render_resource::LoadOp,
    renderer::RenderContext,
    view::{ExtractedView, ViewDepthTexture, ViewTarget},
};
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;

pub struct MainPass3dNode {
    query: QueryState<
        (
            &'static ExtractedCamera,
            &'static RenderPhase<Opaque3d>,
            &'static RenderPhase<AlphaMask3d>,
            &'static RenderPhase<Transparent3d>,
            &'static Camera3d,
            &'static ViewTarget,
            &'static ViewDepthTexture,
            Option<&'static DepthPrepass>,
            Option<&'static NormalPrepass>,
        ),
        With<ExtractedView>,
    >,
}

impl MainPass3dNode {
    pub const IN_VIEW: &'static str = "view";

    pub fn new(world: &mut World) -> Self {
        Self {
            query: world.query_filtered(),
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
        let Ok((
            camera,
            opaque_phase,
            alpha_mask_phase,
            transparent_phase,
            camera_3d,
            target,
            depth,
            depth_prepass,
            normal_prepass,
        )) = self.query.get_manual(world, view_entity) else {
            // No window
            return Ok(());
        };

        // Always run opaque pass to ensure screen is cleared
        {
            // Run the opaque pass, sorted front-to-back
            // NOTE: Scoped to drop the mutable borrow of render_context
            #[cfg(feature = "trace")]
            let _main_opaque_pass_3d_span = info_span!("main_opaque_pass_3d").entered();

            let depth_load_op = if depth_prepass.is_some() || normal_prepass.is_some() {
                // if any prepass runs, it will generate a depth buffer so we should use it,
                // even if only the normal_prepass is used.
                LoadOp::Load
            } else {
                camera_3d.depth_load_op.clone().into()
            };

            render_context
                .render_pass(view_entity)
                .set_label("main_opaque_pass_3d")
                .add_view_target(target)
                .set_color_ops(camera_3d.clear_color.load_op(world), true)
                .set_depth_stencil_attachment(&depth.view)
                .set_depth_ops(depth_load_op, true)
                .begin()
                .set_camera_viewport(camera)
                .render_phase(opaque_phase, world);
        }

        if !alpha_mask_phase.items.is_empty() {
            // Run the alpha mask pass, sorted front-to-back
            // NOTE: Scoped to drop the mutable borrow of render_context
            #[cfg(feature = "trace")]
            let _main_alpha_mask_pass_3d_span = info_span!("main_alpha_mask_pass_3d").entered();

            render_context
                .render_pass(view_entity)
                .set_label("main_alpha_mask_pass_3d")
                .add_view_target(target)
                .set_color_ops(LoadOp::Load, true)
                .set_depth_stencil_attachment(&depth.view)
                .set_depth_ops(LoadOp::Load, true)
                .begin()
                .set_camera_viewport(camera)
                .render_phase(alpha_mask_phase, world);
        }

        if !transparent_phase.items.is_empty() {
            // Run the transparent pass, sorted back-to-front
            // NOTE: Scoped to drop the mutable borrow of render_context
            #[cfg(feature = "trace")]
            let _main_transparent_pass_3d_span = info_span!("main_transparent_pass_3d").entered();

            // NOTE: For the transparent pass we load the depth buffer. There should be no
            // need to write to it, but store is set to `true` as a workaround for issue #3776,
            // https://github.com/bevyengine/bevy/issues/3776
            // so that wgpu does not clear the depth buffer.
            // As the opaque and alpha mask passes run first, opaque meshes can occlude
            // transparent ones.
            render_context
                .render_pass(view_entity)
                .set_label("main_transparent_pass_3d")
                .add_view_target(target)
                .set_color_ops(LoadOp::Load, true)
                .set_depth_stencil_attachment(&depth.view)
                .set_depth_ops(LoadOp::Load, true)
                .begin()
                .set_camera_viewport(camera)
                .render_phase(transparent_phase, world);
        }

        // WebGL2 quirk: if ending with a render pass with a custom viewport, the viewport isn't
        // reset for the next render pass so add an empty render pass without a custom viewport
        #[cfg(feature = "webgl")]
        if camera.viewport.is_some() {
            #[cfg(feature = "trace")]
            let _reset_viewport_pass_3d = info_span!("reset_viewport_pass_3d").entered();

            render_context
                .render_pass(view_entity)
                .set_label("reset_viewport_pass_3d")
                .add_view_target(target)
                .set_color_ops(LoadOp::Load, true)
                .begin();
        }

        Ok(())
    }
}

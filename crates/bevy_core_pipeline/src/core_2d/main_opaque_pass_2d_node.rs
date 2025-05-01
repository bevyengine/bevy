use bevy_ecs::{prelude::World, query::QueryItem};
use bevy_render::{
    camera::ExtractedCamera,
    frame_graph::{
        render_pass_builder::RenderPassBuilder, DepthStencilAttachmentRef, FrameGraph,
        TextureViewInfo, TextureViewRef,
    },
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_phase::{TrackedRenderPass, ViewBinnedRenderPhases},
    render_resource::StoreOp,
    renderer::RenderDevice,
    view::{ExtractedView, ViewDepthTexture, ViewTarget},
};
use tracing::error;
#[cfg(feature = "trace")]
use tracing::info_span;

use super::{AlphaMask2d, Opaque2d};

/// A [`bevy_render::render_graph::Node`] that runs the
/// [`Opaque2d`] [`ViewBinnedRenderPhases`] and [`AlphaMask2d`] [`ViewBinnedRenderPhases`]
#[derive(Default)]
pub struct MainOpaquePass2dNode;
impl ViewNode for MainOpaquePass2dNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ExtractedView,
        &'static ViewTarget,
        &'static ViewDepthTexture,
    );

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        frame_graph: &mut FrameGraph,
        (camera, view, target, depth): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let (Some(opaque_phases), Some(alpha_mask_phases)) = (
            world.get_resource::<ViewBinnedRenderPhases<Opaque2d>>(),
            world.get_resource::<ViewBinnedRenderPhases<AlphaMask2d>>(),
        ) else {
            return Ok(());
        };

        let view_entity = graph.view_entity();
        let (Some(opaque_phase), Some(alpha_mask_phase)) = (
            opaque_phases.get(&view.retained_view_entity),
            alpha_mask_phases.get(&view.retained_view_entity),
        ) else {
            return Ok(());
        };

        let render_device = world.resource::<RenderDevice>();

        let mut builder =
            RenderPassBuilder::new(frame_graph.create_pass_node_bulder("main_opaque_pass_2d"));

        let depth_texture_read = builder.import_and_read_texture(&depth.texture);

        builder
            .add_color_attachment(target)?
            .set_depth_stencil_attachment(&DepthStencilAttachmentRef {
                view_ref: TextureViewRef {
                    texture_ref: depth_texture_read,
                    desc: TextureViewInfo::default(),
                },
                depth_ops: depth.get_depth_ops(StoreOp::Store),
                stencil_ops: None,
            })?
            .set_viewport(camera.viewport.clone());

        let mut tracked_render_pass = TrackedRenderPass::new(&render_device, builder);
        if !opaque_phase.is_empty() {
            if let Err(err) = opaque_phase.render(&mut tracked_render_pass, world, view_entity) {
                error!("Error encountered while rendering the 2d opaque phase {err:?}");
            }
        }

        // Alpha mask draws
        if !alpha_mask_phase.is_empty() {
            if let Err(err) = alpha_mask_phase.render(&mut tracked_render_pass, world, view_entity)
            {
                error!("Error encountered while rendering the 2d alpha mask phase {err:?}");
            }
        }

        Ok(())
    }
}

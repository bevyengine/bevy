use bevy_ecs::{prelude::World, query::QueryItem};
use bevy_render::{
    camera::{ClearColor, ExtractedCamera},
    frame_graph::{
        ColorAttachmentRef, DepthStencilAttachmentRef, FrameGraph, FrameGraphTexture, RenderPass,
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
        (_camera, view, target, depth): QueryItem<'w, Self::ViewQuery>,
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

        let mut builder = frame_graph.create_pass_node_bulder("main_opaque_pass_2d");

        let clear_color_global = world.resource::<ClearColor>();

        let main_texture_key = target.get_main_texture_key(view_entity);

        let Some(main_texture_handle) = builder.read_from_board(&main_texture_key) else {
            return Ok(());
        };

        let main_texture_read = builder.read(main_texture_handle);

        let main_texture_sampled_key = ViewTarget::get_main_texture_sampled(view_entity);

        let Some(main_texture_sampled_handle) = builder.read_from_board(&main_texture_sampled_key)
        else {
            return Ok(());
        };

        let main_texture_sampled_read = builder.read(main_texture_sampled_handle);

        let mut render_pass = RenderPass::default();
        render_pass.add_color_attachment(ColorAttachmentRef {
            view_ref: TextureViewRef {
                texture_ref: main_texture_sampled_read,
                desc: TextureViewInfo::default(),
            },
            resolve_target: Some(TextureViewRef {
                texture_ref: main_texture_read,
                desc: TextureViewInfo::default(),
            }),
            ops: target.get_attachment_operations(),
        });

        let depth_texture = FrameGraphTexture::new_arc_with_texture(&depth.texture);
        let depth_texture_key = ViewDepthTexture::get_depth_texture_key(view_entity);
        let depth_texture_handle = builder.import(&depth_texture_key, depth_texture);
        let depth_texture_read = builder.read(depth_texture_handle);

        render_pass.set_depth_stencil_attachment(DepthStencilAttachmentRef {
            view_ref: TextureViewRef {
                texture_ref: depth_texture_read,
                desc: TextureViewInfo::default(),
            },
            depth_ops: depth.get_depth_ops(StoreOp::Store),
            stencil_ops: None,
        });

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

        tracked_render_pass.finish(render_pass);

        Ok(())
    }
}

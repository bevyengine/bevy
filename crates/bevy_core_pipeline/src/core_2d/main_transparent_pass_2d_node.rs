use bevy_ecs::prelude::*;
use bevy_render::{
    camera::ExtractedCamera,
    frame_graph::{
        BluePrintProvider, DepthStencilAttachmentRef, FrameGraph, RenderPass, TextureViewInfo,
        TextureViewRef,
    },
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_phase::{TrackedRenderPass, ViewSortedRenderPhases},
    render_resource::StoreOp,
    renderer::RenderDevice,
    view::{ExtractedView, ViewDepthTexture, ViewTarget},
};

use tracing::error;
#[cfg(feature = "trace")]
use tracing::info_span;

use super::Transparent2d;

#[derive(Default)]
pub struct MainTransparentPass2dNode {}

impl ViewNode for MainTransparentPass2dNode {
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
        (camera, view, target, depth): bevy_ecs::query::QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let Some(transparent_phases) =
            world.get_resource::<ViewSortedRenderPhases<Transparent2d>>()
        else {
            return Ok(());
        };

        let view_entity = graph.view_entity();
        let Some(transparent_phase) = transparent_phases.get(&view.retained_view_entity) else {
            return Ok(());
        };

        let render_device = world.resource::<RenderDevice>();

        let mut builder = frame_graph.create_pass_node_bulder("main_transparent_pass_2d");

        let mut render_pass = RenderPass::default();

        render_pass.add_color_attachment(target.make_blue_print(&mut builder)?);

        let depth_texture_read = builder.import_and_read_texture(&depth.texture);

        render_pass.set_depth_stencil_attachment(DepthStencilAttachmentRef {
            view_ref: TextureViewRef {
                texture_ref: depth_texture_read,
                desc: TextureViewInfo::default(),
            },
            depth_ops: depth.get_depth_ops(StoreOp::Store),
            stencil_ops: None,
        });

        render_pass.set_viewport(camera.viewport.clone());

        let mut tracked_render_pass = TrackedRenderPass::new(&render_device, builder);

        if !transparent_phase.items.is_empty() {
            if let Err(err) = transparent_phase.render(&mut tracked_render_pass, world, view_entity)
            {
                error!("Error encountered while rendering the transparent 2D phase {err:?}");
            }
        }

        Ok(())
    }
}

use bevy_camera::{MainPassResolutionOverride, Viewport};
use bevy_color::LinearRgba;
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::{
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::{
        LoadOp, Operations, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor, StoreOp,
    },
    renderer::RenderContext,
    view::ViewTarget,
};

use crate::oit::{
    wb_oit::{WbOitResolveBindGroup, WbOitResolvePipelineId},
    WeightedBlendedOitTextures,
};

#[derive(Default)]
pub struct WbOitResolveNode;
impl ViewNode for WbOitResolveNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ViewTarget,
        Option<&'static MainPassResolutionOverride>,
        &'static WbOitResolvePipelineId,
        &'static WbOitResolveBindGroup,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (camera, view_target, resolution_override, pipeline_id, bind_group): QueryItem<
            Self::ViewQuery,
        >,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_id.0) else {
            return Ok(());
        };

        let diagnostics = render_context.diagnostic_recorder();

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("wboit_resolve"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: view_target.main_texture_view(),
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        let pass_span = diagnostics.pass_span(&mut render_pass, "wboit_resolve");

        if let Some(viewport) =
            Viewport::from_viewport_and_override(camera.viewport.as_ref(), resolution_override)
        {
            render_pass.set_camera_viewport(&viewport);
        }

        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group.0, &[]);

        render_pass.draw(0..3, 0..1);

        pass_span.end(&mut render_pass);

        Ok(())
    }
}

#[derive(Default)]
pub struct WbOitClearPassNode;
impl ViewNode for WbOitClearPassNode {
    type ViewQuery = &'static WeightedBlendedOitTextures;

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        wboit_textures: QueryItem<Self::ViewQuery>,
        _world: &World,
    ) -> Result<(), NodeRunError> {
        let pass_descriptor = RenderPassDescriptor {
            label: Some("wboit_clear_pass"),
            color_attachments: &[
                Some(RenderPassColorAttachment {
                    view: &wboit_textures.accum.default_view,
                    ops: Operations {
                        load: LoadOp::Clear(LinearRgba::new(0.0, 0.0, 0.0, 0.0).into()),
                        store: StoreOp::Store,
                    },
                    depth_slice: None,
                    resolve_target: None,
                }),
                Some(RenderPassColorAttachment {
                    view: &wboit_textures.reveal.default_view,
                    ops: Operations {
                        load: LoadOp::Clear(LinearRgba::new(1.0, 0.0, 0.0, 0.0).into()),
                        store: StoreOp::Store,
                    },
                    depth_slice: None,
                    resolve_target: None,
                }),
            ],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        };

        render_context
            .command_encoder()
            .begin_render_pass(&pass_descriptor);

        Ok(())
    }
}

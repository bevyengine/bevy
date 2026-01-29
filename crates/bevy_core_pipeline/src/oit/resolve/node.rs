use bevy_camera::{MainPassResolutionOverride, Viewport};
use bevy_ecs::prelude::*;
use bevy_render::{
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
    render_resource::{BindGroupEntries, PipelineCache, RenderPassDescriptor},
    renderer::{RenderContext, ViewQuery},
    view::{ViewDepthTexture, ViewTarget, ViewUniformOffset},
};

use crate::prepass::DepthPrepass;

use super::{OitResolveBindGroup, OitResolvePipeline, OitResolvePipelineId};

pub fn oit_resolve(
    view: ViewQuery<(
        &ExtractedCamera,
        &ViewTarget,
        &ViewUniformOffset,
        &OitResolvePipelineId,
        &ViewDepthTexture,
        Option<&MainPassResolutionOverride>,
    )>,
    resolve_pipeline: Option<Res<OitResolvePipeline>>,
    bind_group: Option<Res<OitResolveBindGroup>>,
    pipeline_cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let (camera, view_target, view_uniform, oit_resolve_pipeline_id, depth, resolution_override) =
        view.into_inner();

/// The node that executes the OIT resolve pass.
#[derive(Default)]
pub struct OitResolveNode;
impl ViewNode for OitResolveNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static ViewUniformOffset,
        &'static OitResolvePipelineId,
        &'static ViewDepthTexture,
        Option<&'static MainPassResolutionOverride>,
        Has<DepthPrepass>,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (
            camera,
            view_target,
            view_uniform,
            oit_resolve_pipeline_id,
            depth,
            resolution_override,
            depth_prepass,
        ): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        // This *must* run after main_transparent_pass_3d to reset the `oit_atomic_counter` and `oit_heads` buffer
        // Otherwise transparent pass will construct a corrupted linked list(can have circular references which causes infinite loop and device lost) on the next pass

        let resolve_pipeline = world.get_resource::<OitResolvePipeline>().unwrap();

    let mut render_pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("oit_resolve"),
        color_attachments: &[Some(view_target.get_color_attachment())],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    let pass_span = diagnostics.pass_span(&mut render_pass, "oit_resolve");

            let diagnostics = render_context.diagnostic_recorder();

            let depth_bind_group = if !depth_prepass {
                Some(
                    render_context.render_device().create_bind_group(
                        "oit_resolve_depth_bind_group",
                        &pipeline_cache
                            .get_bind_group_layout(&resolve_pipeline.oit_depth_bind_group_layout),
                        &BindGroupEntries::single(depth.view()),
                    ),
                )
            } else {
                None
            };

            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("oit_resolve"),
                color_attachments: &[Some(view_target.get_color_attachment())],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            let pass_span = diagnostics.pass_span(&mut render_pass, "oit_resolve");

            if let Some(viewport) =
                Viewport::from_viewport_and_override(camera.viewport.as_ref(), resolution_override)
            {
                render_pass.set_camera_viewport(&viewport);
            }

            render_pass.set_render_pipeline(pipeline);
            render_pass.set_bind_group(0, bind_group, &[view_uniform.offset]);
            if let Some(depth_bind_group) = &depth_bind_group {
                render_pass.set_bind_group(1, depth_bind_group, &[]);
            }
            render_pass.draw(0..3, 0..1);

            pass_span.end(&mut render_pass);
        }

        Ok(())
    }

    render_pass.set_render_pipeline(pipeline);
    render_pass.set_bind_group(0, &bind_group, &[view_uniform.offset]);
    render_pass.set_bind_group(1, &depth_bind_group, &[]);

    render_pass.draw(0..3, 0..1);

    pass_span.end(&mut render_pass);
}

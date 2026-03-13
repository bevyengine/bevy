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
        Has<DepthPrepass>,
    )>,
    resolve_pipeline: Option<Res<OitResolvePipeline>>,
    bind_group: Option<Res<OitResolveBindGroup>>,
    pipeline_cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let (
        camera,
        view_target,
        view_uniform,
        oit_resolve_pipeline_id,
        depth,
        resolution_override,
        depth_prepass,
    ) = view.into_inner();

    // This *must* run after main_transparent_pass_3d to reset the `oit_atomic_counter` and `oit_heads` buffer
    // Otherwise transparent pass will construct a corrupted linked list(can have circular references which causes infinite loop and device lost) on the next pass.
    let Some(resolve_pipeline) = resolve_pipeline else {
        return;
    };
    let Some(bind_group) = bind_group else {
        return;
    };
    let Some(pipeline) = pipeline_cache.get_render_pipeline(oit_resolve_pipeline_id.0) else {
        return;
    };

    let depth_bind_group = if !depth_prepass {
        Some(ctx.render_device().create_bind_group(
            "oit_resolve_depth_bind_group",
            &pipeline_cache.get_bind_group_layout(&resolve_pipeline.oit_depth_bind_group_layout),
            &BindGroupEntries::single(depth.view()),
        ))
    } else {
        None
    };

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();

    let mut render_pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("oit_resolve"),
        color_attachments: &[Some(view_target.get_color_attachment())],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    let pass_span = diagnostics.pass_span(&mut render_pass, "oit_resolve");

    if let Some(viewport) =
        Viewport::from_viewport_and_override(camera.viewport.as_ref(), resolution_override)
    {
        render_pass.set_camera_viewport(&viewport);
    }

    render_pass.set_render_pipeline(pipeline);
    render_pass.set_bind_group(0, &bind_group, &[view_uniform.offset]);
    if let Some(depth_bind_group) = &depth_bind_group {
        render_pass.set_bind_group(1, depth_bind_group, &[]);
    }
    render_pass.draw(0..3, 0..1);

    pass_span.end(&mut render_pass);
}

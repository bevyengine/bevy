use crate::{
    core_3d::Opaque3d,
    skybox::{SkyboxBindGroup, SkyboxPipelineId},
};
use bevy_camera::{MainPassResolutionOverride, Viewport};
use bevy_ecs::prelude::*;
use bevy_log::error;
#[cfg(feature = "trace")]
use bevy_log::info_span;
use bevy_render::{
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
    render_phase::ViewBinnedRenderPhases,
    render_resource::{PipelineCache, RenderPassDescriptor, StoreOp},
    renderer::{RenderContext, ViewQuery},
    view::{ExtractedMultiview, ExtractedView, ViewDepthTexture, ViewTarget, ViewUniformOffset},
};
use core::num::NonZeroU32;

use super::AlphaMask3d;

pub fn main_opaque_pass_3d(
    world: &World,
    view: ViewQuery<(
        &ExtractedCamera,
        &ExtractedView,
        &ViewTarget,
        &ViewDepthTexture,
        Option<&SkyboxPipelineId>,
        Option<&SkyboxBindGroup>,
        &ViewUniformOffset,
        Option<&MainPassResolutionOverride>,
        Option<&ExtractedMultiview>,
    )>,
    opaque_phases: Res<ViewBinnedRenderPhases<Opaque3d>>,
    alpha_mask_phases: Res<ViewBinnedRenderPhases<AlphaMask3d>>,
    pipeline_cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let view_entity = view.entity();

    let (
        camera,
        extracted_view,
        target,
        depth,
        skybox_pipeline,
        skybox_bind_group,
        view_uniform_offset,
        resolution_override,
        multiview,
    ) = view.into_inner();

    let (Some(opaque_phase), Some(alpha_mask_phase)) = (
        opaque_phases.get(&extracted_view.retained_view_entity),
        alpha_mask_phases.get(&extracted_view.retained_view_entity),
    ) else {
        return;
    };

    #[cfg(feature = "trace")]
    let _main_opaque_pass_3d_span = info_span!("main_opaque_pass_3d").entered();

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();

    let color_attachments = [Some(target.get_color_attachment())];
    let depth_stencil_attachment = Some(depth.get_attachment(StoreOp::Store));

    let mut render_pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("main_opaque_pass_3d"),
        color_attachments: &color_attachments,
        depth_stencil_attachment,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    let pass_span = diagnostics.pass_span(&mut render_pass, "main_opaque_pass_3d");

    if let Some(viewport) =
        Viewport::from_viewport_and_override(camera.viewport.as_ref(), resolution_override)
    {
        render_pass.set_camera_viewport(&viewport);
    }

    if !opaque_phase.is_empty() {
        #[cfg(feature = "trace")]
        let _opaque_main_pass_3d_span = info_span!("opaque_main_pass_3d").entered();
        if let Err(err) = opaque_phase.render(&mut render_pass, world, view_entity) {
            error!("Error encountered while rendering the opaque phase {err:?}");
        }
    }

    if !alpha_mask_phase.is_empty() {
        #[cfg(feature = "trace")]
        let _alpha_mask_main_pass_3d_span = info_span!("alpha_mask_main_pass_3d").entered();
        if let Err(err) = alpha_mask_phase.render(&mut render_pass, world, view_entity) {
            error!("Error encountered while rendering the alpha mask phase {err:?}");
        }
    }

    pass_span.end(&mut render_pass);
    drop(render_pass);

    // L7d: skybox runs in its own broadcast pass after the opaque + alpha-mask
    // draws. The skybox cubemap is shared across eyes; per-eye view matrices
    // (sampled via `view()` from `@builtin(view_index)`) give each eye the
    // correct ray direction, so one broadcast draw fills every layer of the
    // multi-layer color + depth attachments. Re-deriving the attachments
    // through `target.get_color_attachment()` / `depth.get_attachment(...)`
    // hits the second-call `is_first_call` latch and returns `LoadOp::Load`,
    // preserving the opaque + alpha-mask output. Surrounding main_opaque_pass
    // draws stay in their existing single-pass shape (Shape A1 extract,
    // parallel to session 23's bg-motion-vectors broadcast extract).
    //
    // The mask is `(1 << view_count) - 1` (one bit per eye); computed via
    // `u32::MAX >> (32 - view_count)` to avoid the shift overflow that
    // `1 << 32` would hit at the `MAX_VIEW_COUNT` cap.
    if let (Some(skybox_pipeline), Some(SkyboxBindGroup(skybox_bind_group))) =
        (skybox_pipeline, skybox_bind_group)
        && let Some(pipeline) = pipeline_cache.get_render_pipeline(skybox_pipeline.0)
    {
        let view_count = multiview.map_or(1, |m| m.subviews.len() as u32);
        let multiview_mask = if view_count > 1 {
            NonZeroU32::new(u32::MAX >> (32 - view_count))
        } else {
            None
        };

        let skybox_color_attachments = [Some(target.get_color_attachment())];
        let skybox_depth_attachment = Some(depth.get_attachment(StoreOp::Store));

        let mut skybox_pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("skybox_broadcast"),
            color_attachments: &skybox_color_attachments,
            depth_stencil_attachment: skybox_depth_attachment,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask,
        });
        let skybox_span = diagnostics.pass_span(&mut skybox_pass, "skybox_broadcast");

        if let Some(viewport) =
            Viewport::from_viewport_and_override(camera.viewport.as_ref(), resolution_override)
        {
            skybox_pass.set_camera_viewport(&viewport);
        }

        skybox_pass.set_render_pipeline(pipeline);
        skybox_pass.set_bind_group(
            0,
            &skybox_bind_group.0,
            &[view_uniform_offset.offset, skybox_bind_group.1],
        );
        skybox_pass.draw(0..3, 0..1);

        skybox_span.end(&mut skybox_pass);
    }
}

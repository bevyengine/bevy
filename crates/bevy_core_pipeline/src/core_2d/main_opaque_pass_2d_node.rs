use crate::core_2d::Opaque2d;
use bevy_ecs::prelude::*;
use bevy_render::{
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
    render_phase::ViewBinnedRenderPhases,
    render_resource::{RenderPassDescriptor, StoreOp},
    renderer::{RenderContext, ViewQuery},
    view::{ExtractedView, ViewDepthTexture, ViewTarget},
};
use tracing::error;
#[cfg(feature = "trace")]
use tracing::info_span;

use super::AlphaMask2d;

pub fn main_opaque_pass_2d(
    world: &World,
    view: ViewQuery<(
        &ExtractedCamera,
        &ExtractedView,
        &ViewTarget,
        &ViewDepthTexture,
    )>,
    opaque_phases: Res<ViewBinnedRenderPhases<Opaque2d>>,
    alpha_mask_phases: Res<ViewBinnedRenderPhases<AlphaMask2d>>,
    mut ctx: RenderContext,
) {
    let view_entity = view.entity();
    let (camera, extracted_view, target, depth) = view.into_inner();

    let (Some(opaque_phase), Some(alpha_mask_phase)) = (
        opaque_phases.get(&extracted_view.retained_view_entity),
        alpha_mask_phases.get(&extracted_view.retained_view_entity),
    ) else {
        return;
    };

    if opaque_phase.is_empty() && alpha_mask_phase.is_empty() {
        return;
    }

    #[cfg(feature = "trace")]
    let _span = info_span!("main_opaque_pass_2d").entered();

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();

    let color_attachments = [Some(target.get_color_attachment())];
    let depth_stencil_attachment = Some(depth.get_attachment(StoreOp::Store));

    let mut render_pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("main_opaque_pass_2d"),
        color_attachments: &color_attachments,
        depth_stencil_attachment,
        timestamp_writes: None,
        occlusion_query_set: None,
    });
    let pass_span = diagnostics.pass_span(&mut render_pass, "main_opaque_pass_2d");

    if let Some(viewport) = camera.viewport.as_ref() {
        render_pass.set_camera_viewport(viewport);
    }

    if !opaque_phase.is_empty() {
        #[cfg(feature = "trace")]
        let _opaque_span = info_span!("opaque_main_pass_2d").entered();
        if let Err(err) = opaque_phase.render(&mut render_pass, world, view_entity) {
            error!("Error encountered while rendering the 2d opaque phase {err:?}");
        }
    }

    if !alpha_mask_phase.is_empty() {
        #[cfg(feature = "trace")]
        let _alpha_mask_span = info_span!("alpha_mask_main_pass_2d").entered();
        if let Err(err) = alpha_mask_phase.render(&mut render_pass, world, view_entity) {
            error!("Error encountered while rendering the 2d alpha mask phase {err:?}");
        }
    }

    pass_span.end(&mut render_pass);
}

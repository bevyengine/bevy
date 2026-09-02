use crate::{
    core_3d::{
        render_opaque_pass_3d, render_transparent_pass_3d, AlphaMask3d, Opaque3d, Transparent3d,
    },
    oit::{resolve::OitResolvePipelineId, OrderIndependentTransparencySettings},
    skybox::{SkyboxBindGroup, SkyboxPipelineId},
};
use bevy_camera::{MainPassResolutionOverride, Viewport};
use bevy_ecs::prelude::*;
#[cfg(feature = "trace")]
use bevy_log::info_span;
use bevy_render::{
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
    render_phase::{ViewBinnedRenderPhases, ViewSortedRenderPhases},
    render_resource::{PipelineCache, RenderPassDescriptor, StoreOp},
    renderer::{RenderContext, ViewQuery},
    view::{ExtractedView, ViewDepthStencilTexture, ViewTarget, ViewUniformOffset},
};

pub fn main_merged_pass_3d(
    world: &World,
    view: ViewQuery<(
        &ExtractedCamera,
        &ExtractedView,
        &ViewTarget,
        &ViewDepthStencilTexture,
        Option<&SkyboxPipelineId>,
        Option<&SkyboxBindGroup>,
        &ViewUniformOffset,
        Option<&MainPassResolutionOverride>,
        Has<OrderIndependentTransparencySettings>,
        Option<&OitResolvePipelineId>,
    )>,
    opaque_phases: Res<ViewBinnedRenderPhases<Opaque3d>>,
    alpha_mask_phases: Res<ViewBinnedRenderPhases<AlphaMask3d>>,
    transparent_phases: Res<ViewSortedRenderPhases<Transparent3d>>,
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
        has_oit,
        oit_resolve_pipeline_id,
    ) = view.into_inner();

    let (Some(opaque_phase), Some(alpha_mask_phase), Some(transparent_phase)) = (
        opaque_phases.get(&extracted_view.retained_view_entity),
        alpha_mask_phases.get(&extracted_view.retained_view_entity),
        transparent_phases.get(&extracted_view.retained_view_entity),
    ) else {
        return;
    };

    #[cfg(feature = "trace")]
    let _span = info_span!("main_merged_pass_3d").entered();

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();

    let color_attachments = [Some(target.get_color_attachment())];
    let depth_stencil_attachment = Some(depth.get_attachment(StoreOp::Store));

    let mut render_pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("main_merged_pass_3d"),
        color_attachments: &color_attachments,
        depth_stencil_attachment,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    let pass_span = diagnostics.pass_span(&mut render_pass, "main_merged_pass_3d");

    if let Some(viewport) =
        Viewport::from_viewport_and_override(camera.viewport.as_ref(), resolution_override)
    {
        render_pass.set_camera_viewport(&viewport);
    }

    render_opaque_pass_3d(
        &mut render_pass,
        world,
        view_entity,
        opaque_phase,
        alpha_mask_phase,
        skybox_pipeline,
        skybox_bind_group,
        view_uniform_offset,
        &pipeline_cache,
    );

    'b: {
        if !transparent_phase.items.is_empty() {
            if has_oit {
                // We can't run transparent phase if OitResolvePipelineId is not ready
                // Otherwise we will write to `oit_atomic_counter` and `oit_heads` buffer without resetting them
                // which causes corrupted linked list(can have circular references) on the next pass
                let Some(oit_resolve_pipeline_id) = oit_resolve_pipeline_id else {
                    break 'b;
                };
                let pipeline_cache = world.resource::<PipelineCache>();
                if pipeline_cache
                    .get_render_pipeline(oit_resolve_pipeline_id.0)
                    .is_none()
                {
                    break 'b;
                }
            }

            render_transparent_pass_3d(&mut render_pass, world, view_entity, transparent_phase);
        }
    }

    pass_span.end(&mut render_pass);
}

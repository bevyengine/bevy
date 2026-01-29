use bevy_camera::{MainPassResolutionOverride, Viewport};
use bevy_ecs::prelude::*;
use bevy_render::{
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
    occlusion_culling::OcclusionCulling,
    render_phase::ViewBinnedRenderPhases,
    render_resource::{PipelineCache, RenderPassDescriptor, StoreOp},
    renderer::{RenderContext, ViewQuery},
    view::{ExtractedView, NoIndirectDrawing, ViewDepthTexture, ViewUniformOffset},
};
use tracing::error;
#[cfg(feature = "trace")]
use tracing::info_span;

use crate::skybox::prepass::{RenderSkyboxPrepassPipeline, SkyboxPrepassBindGroup};

use super::{
    AlphaMask3dPrepass, DeferredPrepass, Opaque3dPrepass, PreviousViewUniformOffset,
    ViewPrepassTextures,
};

/// Type alias for the prepass view query.
type PrepassViewQueryData = (
    (
        &'static ExtractedCamera,
        &'static ExtractedView,
        &'static ViewDepthTexture,
        &'static ViewPrepassTextures,
        &'static ViewUniformOffset,
    ),
    (
        Option<&'static DeferredPrepass>,
        Option<&'static RenderSkyboxPrepassPipeline>,
        Option<&'static SkyboxPrepassBindGroup>,
        Option<&'static PreviousViewUniformOffset>,
        Option<&'static MainPassResolutionOverride>,
    ),
    (
        Has<OcclusionCulling>,
        Has<NoIndirectDrawing>,
        Has<DeferredPrepass>,
    ),
);

pub fn early_prepass(
    world: &World,
    view: ViewQuery<PrepassViewQueryData>,
    opaque_prepass_phases: Res<ViewBinnedRenderPhases<Opaque3dPrepass>>,
    alpha_mask_prepass_phases: Res<ViewBinnedRenderPhases<AlphaMask3dPrepass>>,
    pipeline_cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let view_entity = view.entity();
    let (
        (camera, extracted_view, view_depth_texture, view_prepass_textures, view_uniform_offset),
        (
            deferred_prepass,
            skybox_prepass_pipeline,
            skybox_prepass_bind_group,
            view_prev_uniform_offset,
            resolution_override,
        ),
        (_, _, has_deferred),
    ) = view.into_inner();

    run_prepass_system(
        world,
        view_entity,
        camera,
        extracted_view,
        view_depth_texture,
        view_prepass_textures,
        view_uniform_offset,
        deferred_prepass,
        skybox_prepass_pipeline,
        skybox_prepass_bind_group,
        view_prev_uniform_offset,
        resolution_override,
        has_deferred,
        &opaque_prepass_phases,
        &alpha_mask_prepass_phases,
        &pipeline_cache,
        &mut ctx,
        "early prepass",
    );
}

pub fn late_prepass(
    world: &World,
    view: ViewQuery<PrepassViewQueryData>,
    opaque_prepass_phases: Res<ViewBinnedRenderPhases<Opaque3dPrepass>>,
    alpha_mask_prepass_phases: Res<ViewBinnedRenderPhases<AlphaMask3dPrepass>>,
    pipeline_cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let view_entity = view.entity();
    let (
        (camera, extracted_view, view_depth_texture, view_prepass_textures, view_uniform_offset),
        (
            deferred_prepass,
            skybox_prepass_pipeline,
            skybox_prepass_bind_group,
            view_prev_uniform_offset,
            resolution_override,
        ),
        (occlusion_culling, no_indirect_drawing, has_deferred),
    ) = view.into_inner();

    if !occlusion_culling || no_indirect_drawing {
        return;
    }

    run_prepass_system(
        world,
        view_entity,
        camera,
        extracted_view,
        view_depth_texture,
        view_prepass_textures,
        view_uniform_offset,
        deferred_prepass,
        skybox_prepass_pipeline,
        skybox_prepass_bind_group,
        view_prev_uniform_offset,
        resolution_override,
        has_deferred,
        &opaque_prepass_phases,
        &alpha_mask_prepass_phases,
        &pipeline_cache,
        &mut ctx,
        "late prepass",
    );
}

/// Shared implementation for prepass systems.
#[expect(
    clippy::too_many_arguments,
    reason = "render system with many view components"
)]
fn run_prepass_system(
    world: &World,
    view_entity: Entity,
    camera: &ExtractedCamera,
    extracted_view: &ExtractedView,
    view_depth_texture: &ViewDepthTexture,
    view_prepass_textures: &ViewPrepassTextures,
    view_uniform_offset: &ViewUniformOffset,
    deferred_prepass: Option<&DeferredPrepass>,
    skybox_prepass_pipeline: Option<&RenderSkyboxPrepassPipeline>,
    skybox_prepass_bind_group: Option<&SkyboxPrepassBindGroup>,
    view_prev_uniform_offset: Option<&PreviousViewUniformOffset>,
    resolution_override: Option<&MainPassResolutionOverride>,
    has_deferred: bool,
    opaque_prepass_phases: &ViewBinnedRenderPhases<Opaque3dPrepass>,
    alpha_mask_prepass_phases: &ViewBinnedRenderPhases<AlphaMask3dPrepass>,
    pipeline_cache: &PipelineCache,
    ctx: &mut RenderContext,
    label: &'static str,
) {
    // If we're using deferred rendering, there will be a deferred prepass
    // instead of this one. Just bail out so we don't have to bother looking at
    // the empty bins.
    if has_deferred {
        return;
    }

    let (Some(opaque_prepass_phase), Some(alpha_mask_prepass_phase)) = (
        opaque_prepass_phases.get(&extracted_view.retained_view_entity),
        alpha_mask_prepass_phases.get(&extracted_view.retained_view_entity),
    ) else {
        return;
    };

    #[cfg(feature = "trace")]
    let _prepass_span = info_span!("prepass").entered();

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();

    let mut color_attachments = vec![
        view_prepass_textures
            .normal
            .as_ref()
            .map(|normals_texture| normals_texture.get_attachment()),
        view_prepass_textures
            .motion_vectors
            .as_ref()
            .map(|motion_vectors_texture| motion_vectors_texture.get_attachment()),
        // Use None in place of deferred attachments
        None,
        None,
    ];

    // If all color attachments are none: clear the color attachment list so that no fragment shader is required
    if color_attachments.iter().all(Option::is_none) {
        color_attachments.clear();
    }

    let depth_stencil_attachment = Some(view_depth_texture.get_attachment(StoreOp::Store));

    let mut render_pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some(label),
        color_attachments: &color_attachments,
        depth_stencil_attachment,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    let pass_span = diagnostics.pass_span(&mut render_pass, label);

    if let Some(viewport) =
        Viewport::from_viewport_and_override(camera.viewport.as_ref(), resolution_override)
    {
        render_pass.set_camera_viewport(&viewport);
    }

    if !opaque_prepass_phase.is_empty() {
        #[cfg(feature = "trace")]
        let _opaque_prepass_span = info_span!("opaque_prepass").entered();
        if let Err(err) = opaque_prepass_phase.render(&mut render_pass, world, view_entity) {
            error!("Error encountered while rendering the opaque prepass phase {err:?}");
        }
    }

    if !alpha_mask_prepass_phase.is_empty() {
        #[cfg(feature = "trace")]
        let _alpha_mask_prepass_span = info_span!("alpha_mask_prepass").entered();
        if let Err(err) = alpha_mask_prepass_phase.render(&mut render_pass, world, view_entity) {
            error!("Error encountered while rendering the alpha mask prepass phase {err:?}");
        }
    }

    if let (
        Some(skybox_prepass_pipeline),
        Some(skybox_prepass_bind_group),
        Some(view_prev_uniform_offset),
    ) = (
        skybox_prepass_pipeline,
        skybox_prepass_bind_group,
        view_prev_uniform_offset,
    ) && let Some(pipeline) = pipeline_cache.get_render_pipeline(skybox_prepass_pipeline.0)
    {
        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(
            0,
            &skybox_prepass_bind_group.0,
            &[view_uniform_offset.offset, view_prev_uniform_offset.offset],
        );
        render_pass.draw(0..3, 0..1);
    }

    pass_span.end(&mut render_pass);
    drop(render_pass);

    if deferred_prepass.is_none()
        && let Some(prepass_depth_texture) = &view_prepass_textures.depth
    {
        ctx.command_encoder().copy_texture_to_texture(
            view_depth_texture.texture.as_image_copy(),
            prepass_depth_texture.texture.texture.as_image_copy(),
            view_prepass_textures.size,
        );
    }
}

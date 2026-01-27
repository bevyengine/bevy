use super::{
    prepare::DlssRenderContext, Dlss, DlssRayReconstructionFeature, DlssSuperResolutionFeature,
    ViewDlssRayReconstructionTextures,
};
use bevy_camera::MainPassResolutionOverride;
use bevy_core_pipeline::prepass::ViewPrepassTextures;
use bevy_ecs::system::{Query, Res};
use bevy_render::{
    camera::TemporalJitter,
    diagnostic::RecordDiagnostics,
    renderer::{RenderAdapter, RenderContext, ViewQuery},
    view::ViewTarget,
};
use dlss_wgpu::{
    ray_reconstruction::{
        DlssRayReconstructionRenderParameters, DlssRayReconstructionSpecularGuide,
    },
    super_resolution::{DlssSuperResolutionExposure, DlssSuperResolutionRenderParameters},
};

pub fn dlss_super_resolution(
    view: ViewQuery<(
        &Dlss<DlssSuperResolutionFeature>,
        &DlssRenderContext<DlssSuperResolutionFeature>,
        &MainPassResolutionOverride,
        &TemporalJitter,
        &ViewTarget,
        &ViewPrepassTextures,
    )>,
    adapter: Res<RenderAdapter>,
    mut ctx: RenderContext,
) {
    let (dlss, dlss_context, resolution_override, temporal_jitter, view_target, prepass_textures) =
        view.into_inner();

    let (Some(prepass_depth_texture), Some(prepass_motion_vectors_texture)) =
        (&prepass_textures.depth, &prepass_textures.motion_vectors)
    else {
        return;
    };

    let view_target = view_target.post_process_write();

    let render_resolution = resolution_override.0;
    let render_parameters = DlssSuperResolutionRenderParameters {
        color: &view_target.source,
        depth: &prepass_depth_texture.texture.default_view,
        motion_vectors: &prepass_motion_vectors_texture.texture.default_view,
        exposure: DlssSuperResolutionExposure::Automatic, // TODO
        bias: None,                                       // TODO
        dlss_output: &view_target.destination,
        reset: dlss.reset,
        jitter_offset: -temporal_jitter.offset,
        partial_texture_size: Some(render_resolution),
        motion_vector_scale: Some(-render_resolution.as_vec2()),
    };

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let time_span = diagnostics.time_span(ctx.command_encoder(), "dlss_super_resolution");

    let command_encoder = ctx.command_encoder();
    let mut dlss_context = dlss_context.context.lock().unwrap();

    command_encoder.push_debug_group("dlss_super_resolution");

    let dlss_command_buffer = dlss_context
        .render(render_parameters, command_encoder, &adapter)
        .expect("Failed to render DLSS Super Resolution");

    command_encoder.pop_debug_group();
    ctx.add_command_buffer(dlss_command_buffer);
    time_span.end(ctx.command_encoder());
}

pub fn dlss_ray_reconstruction(
    view: ViewQuery<(
        &Dlss<DlssRayReconstructionFeature>,
        &DlssRenderContext<DlssRayReconstructionFeature>,
        &MainPassResolutionOverride,
        &TemporalJitter,
        &ViewTarget,
        &ViewPrepassTextures,
        &ViewDlssRayReconstructionTextures,
    )>,
    adapter: Res<RenderAdapter>,
    mut ctx: RenderContext,
) {
    let (
        dlss,
        dlss_context,
        resolution_override,
        temporal_jitter,
        view_target,
        prepass_textures,
        ray_reconstruction_textures,
    ) = view.into_inner();

    let (Some(prepass_depth_texture), Some(prepass_motion_vectors_texture)) =
        (&prepass_textures.depth, &prepass_textures.motion_vectors)
    else {
        return;
    };

    let view_target = view_target.post_process_write();

    let render_resolution = resolution_override.0;
    let render_parameters = DlssRayReconstructionRenderParameters {
        diffuse_albedo: &ray_reconstruction_textures.diffuse_albedo.default_view,
        specular_albedo: &ray_reconstruction_textures.specular_albedo.default_view,
        normals: &ray_reconstruction_textures.normal_roughness.default_view,
        roughness: None,
        color: &view_target.source,
        depth: &prepass_depth_texture.texture.default_view,
        motion_vectors: &prepass_motion_vectors_texture.texture.default_view,
        specular_guide: DlssRayReconstructionSpecularGuide::SpecularMotionVectors(
            &ray_reconstruction_textures
                .specular_motion_vectors
                .default_view,
        ),
        screen_space_subsurface_scattering_guide: None, // TODO
        bias: None,                                     // TODO
        dlss_output: &view_target.destination,
        reset: dlss.reset,
        jitter_offset: -temporal_jitter.offset,
        partial_texture_size: Some(render_resolution),
        motion_vector_scale: Some(-render_resolution.as_vec2()),
    };

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let time_span = diagnostics.time_span(ctx.command_encoder(), "dlss_ray_reconstruction");

    let command_encoder = ctx.command_encoder();
    let mut dlss_context = dlss_context.context.lock().unwrap();

    command_encoder.push_debug_group("dlss_ray_reconstruction");

    let dlss_command_buffer = dlss_context
        .render(render_parameters, command_encoder, &adapter)
        .expect("Failed to render DLSS Ray Reconstruction");

    command_encoder.pop_debug_group();
    ctx.add_command_buffer(dlss_command_buffer);
    time_span.end(ctx.command_encoder());
}

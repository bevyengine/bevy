use super::{prepare::MetalFxRenderContext, MetalFx, MetalFxTemporalDenoisedScalerFeature};
use crate::ray_reconstruction::ViewRayReconstructionGuideTextures;
use bevy_camera::MainPassResolutionOverride;
use bevy_core_pipeline::prepass::ViewPrepassTextures;
use bevy_render::{
    camera::TemporalJitter,
    diagnostic::RecordDiagnostics,
    renderer::{RenderContext, ViewQuery},
    view::ViewTarget,
};
use metalfx_wgpu::MetalFxTemporalDenoisedScalerRenderParameters;

pub fn metal_fx_temporal_denoised_scaler(
    view: ViewQuery<(
        &MetalFx<MetalFxTemporalDenoisedScalerFeature>,
        &MetalFxRenderContext<MetalFxTemporalDenoisedScalerFeature>,
        &MainPassResolutionOverride,
        &TemporalJitter,
        &ViewTarget,
        &ViewPrepassTextures,
        &ViewRayReconstructionGuideTextures,
    )>,
    mut ctx: RenderContext,
) {
    let (
        metal_fx,
        metal_fx_context,
        resolution_override,
        temporal_jitter,
        view_target,
        prepass_textures,
        guide_textures,
    ) = view.into_inner();

    let (Some(prepass_depth_texture), Some(prepass_motion_vectors_texture)) =
        (&prepass_textures.depth, &prepass_textures.motion_vectors)
    else {
        return;
    };

    let view_target = view_target.post_process_write();

    let render_resolution = resolution_override.0;
    let render_parameters = MetalFxTemporalDenoisedScalerRenderParameters {
        diffuse_albedo: &guide_textures.diffuse_albedo.default_view,
        specular_albedo: &guide_textures.specular_albedo.default_view,
        normals: &guide_textures.normal_roughness.default_view,
        roughness: &guide_textures.roughness.default_view,
        color: &view_target.source,
        // Depth32Float prepass. The Solari guide depth is an R32Float copy (with virtual depth for
        // mirrors) and is not a depth texture, which MetalFX requires.
        depth: &prepass_depth_texture.texture.default_view,
        motion_vectors: &prepass_motion_vectors_texture.texture.default_view,
        output: &view_target.destination,
        reset: metal_fx.reset,
        jitter_offset: (-temporal_jitter.offset).to_array(),
        motion_vector_scale: Some((-render_resolution.as_vec2()).to_array()),
        pre_exposure: 1.0,
        specular_hit_distance: None,
        reactive_mask: None,
        denoise_strength_mask: None,
        transparency_overlay: None,
        exposure: None,
    };

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let time_span =
        diagnostics.time_span(ctx.command_encoder(), "metal_fx_temporal_denoised_scaler");

    let mut metal_fx_context = metal_fx_context.context.lock().unwrap();
    let metal_fx_command_buffer = metal_fx_context
        .render(render_parameters, ctx.command_encoder())
        .expect("Failed to render MetalFX Temporal Denoised Scaler");

    ctx.add_command_buffer(metal_fx_command_buffer);
    time_span.end(ctx.command_encoder());
}

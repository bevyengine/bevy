use crate::{
    blit::{BlitPipeline, DisplayTargetUniform},
    upscaling::{gamut_matrix, ViewUpscalingPipeline},
};
use bevy_camera::{CameraOutputMode, ClearColor, ClearColorConfig};
use bevy_color::LinearRgba;
use bevy_ecs::prelude::*;
use bevy_math::Vec3;
use bevy_render::{
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
    extract_component::{ComponentUniforms, DynamicUniformIndex},
    render_resource::{
        BindGroup, BindGroupEntries, BufferId, PipelineCache, RenderPassDescriptor, TextureViewId,
    },
    renderer::{RenderContext, ViewQuery},
    transfer_functions::{pq_inverse_eotf_from_nits, srgb_oetf_extended},
    view::{ViewDisplayTarget, ViewTarget},
};
use bevy_window::{ResolvedDisplayTarget, TransferFunction, SCRGB_REFERENCE_WHITE_NITS};

#[derive(Default)]
pub struct UpscalingBindGroupCache {
    cached: Option<(TextureViewId, BindGroup)>,
    display_target: Option<(BufferId, BindGroup)>,
}

pub fn upscaling(
    view: ViewQuery<(
        &ViewTarget,
        &ViewUpscalingPipeline,
        Option<&ExtractedCamera>,
        Option<&ViewDisplayTarget>,
        Option<&DynamicUniformIndex<DisplayTargetUniform>>,
    )>,
    pipeline_cache: Res<PipelineCache>,
    blit_pipeline: Res<BlitPipeline>,
    display_target_uniforms: Res<ComponentUniforms<DisplayTargetUniform>>,
    clear_color_global: Res<ClearColor>,
    mut cache: Local<UpscalingBindGroupCache>,
    mut ctx: RenderContext,
) {
    let (target, upscaling_target, camera, display_target, display_target_index) =
        view.into_inner();
    let cache = &mut *cache;

    let clear_color = if let Some(camera) = camera {
        match camera.output_mode {
            CameraOutputMode::Write { clear_color, .. } => clear_color,
            CameraOutputMode::Skip => return,
        }
    } else {
        ClearColorConfig::Default
    };
    let clear_color = match clear_color {
        ClearColorConfig::Default => Some(clear_color_global.0),
        ClearColorConfig::Custom(color) => Some(color),
        ClearColorConfig::None => None,
    };
    let display_target = display_target.map(|target| target.0).unwrap_or_default();
    let converted_clear_color =
        clear_color.map(|color| encode_clear_color(color.into(), &display_target));

    // texture to be upscaled to the output texture
    let main_texture_view = target.main_texture_view();

    let bind_group = match &mut cache.cached {
        Some((id, bind_group)) if main_texture_view.id() == *id => bind_group,
        cached => {
            let bind_group = blit_pipeline.create_bind_group(
                ctx.render_device(),
                main_texture_view,
                &pipeline_cache,
            );

            let (_, bind_group) = cached.insert((main_texture_view.id(), bind_group));
            bind_group
        }
    };

    let Some(out_attachment) = target.out_texture_color_attachment(converted_clear_color) else {
        return;
    };

    let pass_descriptor = RenderPassDescriptor {
        label: Some("upscaling"),
        color_attachments: &[Some(out_attachment)],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    };

    let Some(pipeline) = pipeline_cache.get_render_pipeline(upscaling_target.0) else {
        // we need to do some work on the swapchain to avoid uninitialized-drawable
        // artifacts (a pink/magenta screen) on Metal platforms
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        ctx.command_encoder().begin_render_pass(&pass_descriptor);
        return;
    };

    let display_target_bind_group = if upscaling_target.1.display_color_space.is_hdr() {
        let (Some(index), Some(buffer), Some(binding)) = (
            display_target_index,
            display_target_uniforms.buffer(),
            display_target_uniforms.binding(),
        ) else {
            return;
        };
        let bind_group = match &mut cache.display_target {
            Some((id, bind_group)) if buffer.id() == *id => bind_group,
            cached => {
                let bind_group = ctx.render_device().create_bind_group(
                    "blit_display_target_bind_group",
                    &pipeline_cache.get_bind_group_layout(&blit_pipeline.display_target_layout),
                    &BindGroupEntries::single(binding),
                );
                &cached.insert((buffer.id(), bind_group)).1
            }
        };
        Some((bind_group, index.index()))
    } else {
        None
    };

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let time_span = diagnostics.time_span(ctx.command_encoder(), "upscaling");

    {
        let mut render_pass = ctx.command_encoder().begin_render_pass(&pass_descriptor);

        if let Some(camera) = camera
            && let Some(viewport) = &camera.viewport
        {
            let size = viewport.physical_size;
            let position = viewport.physical_position;
            render_pass.set_scissor_rect(position.x, position.y, size.x, size.y);
        }

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        if let Some((display_target_bind_group, index)) = display_target_bind_group {
            render_pass.set_bind_group(1, display_target_bind_group, &[index]);
        }
        render_pass.draw(0..3, 0..1);
    }

    time_span.end(ctx.command_encoder());
}

/// Encodes a linear BT.709 clear color the same way the blit encodes pixels.
/// SDR colors are returned unchanged, like the blit's SDR output.
fn encode_clear_color(color: LinearRgba, display_target: &ResolvedDisplayTarget) -> LinearRgba {
    let color_space = display_target.color_space;
    if !color_space.is_hdr() {
        return color;
    }
    let rgb = gamut_matrix(color_space) * Vec3::new(color.red, color.green, color.blue);
    let paper_white = display_target.paper_white_nits;
    let rgb = if color_space.transfer_function() == TransferFunction::Pq {
        (rgb * paper_white).map(pq_inverse_eotf_from_nits)
    } else {
        let scrgb = rgb * (paper_white / SCRGB_REFERENCE_WHITE_NITS);
        if color_space.transfer_function() == TransferFunction::ExtendedSrgb {
            scrgb.map(srgb_oetf_extended)
        } else {
            scrgb
        }
    };
    LinearRgba::new(rgb.x, rgb.y, rgb.z, color.alpha)
}

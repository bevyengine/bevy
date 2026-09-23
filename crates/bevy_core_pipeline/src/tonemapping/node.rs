use crate::tonemapping::{TonemappingLuts, TonemappingPipeline, ViewTonemappingPipeline};

use bevy_ecs::prelude::*;
use bevy_platform::collections::HashMap;
use bevy_render::{
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
    render_asset::RenderAssets,
    render_resource::{
        BindGroup, BindGroupEntries, BufferId, LoadOp, Operations, PipelineCache,
        RenderPassColorAttachment, RenderPassDescriptor, StoreOp, TextureViewId,
    },
    renderer::{RenderContext, ViewQuery},
    texture::{FallbackImage, GpuImage},
    view::{ViewTarget, ViewUniformOffset, ViewUniforms},
};

use super::{get_lut_bindings, Tonemapping};

/// Cached bind group state for tonemapping.
///
/// The system runs once per view with a single shared cache, so entries are
/// keyed by the view's source texture: one slot would be evicted every pass as
/// soon as there were two views.
#[derive(Default)]
pub struct TonemappingBindGroupCache {
    cached: HashMap<TextureViewId, CachedTonemappingBindGroup>,
}

struct CachedTonemappingBindGroup {
    view_uniforms_id: BufferId,
    lut_id: TextureViewId,
    tonemapping: Tonemapping,
    bind_group: BindGroup,
    /// Set when the entry was used this frame; entries that go unused are dropped.
    used: bool,
}

/// Views a cache can hold before unused entries are swept, so that stale
/// entries from resized or removed views do not pile up.
const CACHE_SWEEP_THRESHOLD: usize = 16;

pub fn tonemapping(
    view: ViewQuery<(
        &ExtractedCamera,
        &ViewUniformOffset,
        &ViewTarget,
        &ViewTonemappingPipeline,
        &Tonemapping,
    )>,
    pipeline_cache: Res<PipelineCache>,
    tonemapping_pipeline: Res<TonemappingPipeline>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    fallback_image: Res<FallbackImage>,
    view_uniforms: Res<ViewUniforms>,
    tonemapping_luts: Res<TonemappingLuts>,
    mut cache: Local<TonemappingBindGroupCache>,
    mut ctx: RenderContext,
) {
    let (camera, view_uniform_offset, target, view_tonemapping_pipeline, tonemapping) =
        view.into_inner();

    if !tonemapping.is_enabled() {
        return;
    }

    if !camera.hdr {
        return;
    }

    let Some(pipeline) = pipeline_cache.get_render_pipeline(view_tonemapping_pipeline.0) else {
        return;
    };

    let view_uniforms_buffer = &view_uniforms.uniforms;
    let view_uniforms_id = view_uniforms_buffer.buffer().unwrap().id();

    let post_process = target.post_process_write();
    let source = post_process.source;
    let destination = post_process.destination;

    if cache.cached.len() >= CACHE_SWEEP_THRESHOLD {
        cache
            .cached
            .retain(|_, entry| core::mem::take(&mut entry.used));
    }

    let cached = cache.cached.get(&source.id()).filter(|cached| {
        cached.view_uniforms_id == view_uniforms_id
            // A fallback LUT means the real one had not loaded yet, so try again.
            && cached.lut_id != fallback_image.d3.texture_view.id()
            && cached.tonemapping == *tonemapping
    });
    let bind_group = match cached {
        Some(_) => {
            let cached = cache.cached.get_mut(&source.id()).unwrap();
            cached.used = true;
            &cached.bind_group
        }
        None => {
            let lut_bindings =
                get_lut_bindings(&gpu_images, &tonemapping_luts, tonemapping, &fallback_image);

            let bind_group = ctx.render_device().create_bind_group(
                None,
                &pipeline_cache.get_bind_group_layout(&tonemapping_pipeline.texture_bind_group),
                &BindGroupEntries::sequential((
                    view_uniforms_buffer,
                    source,
                    &tonemapping_pipeline.sampler,
                    lut_bindings.0,
                    lut_bindings.1,
                )),
            );

            let cached = cache
                .cached
                .entry(source.id())
                .insert(CachedTonemappingBindGroup {
                    view_uniforms_id,
                    lut_id: lut_bindings.0.id(),
                    tonemapping: *tonemapping,
                    bind_group,
                    used: true,
                });
            &cached.into_mut().bind_group
        }
    };

    let pass_descriptor = RenderPassDescriptor {
        label: Some("tonemapping"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: destination,
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Default::default()), // TODO shouldn't need to be cleared
                store: StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    };

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let time_span = diagnostics.time_span(ctx.command_encoder(), "tonemapping");

    {
        let mut render_pass = ctx.command_encoder().begin_render_pass(&pass_descriptor);

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, bind_group, &[view_uniform_offset.offset]);
        render_pass.draw(0..3, 0..1);
    }

    time_span.end(ctx.command_encoder());
}

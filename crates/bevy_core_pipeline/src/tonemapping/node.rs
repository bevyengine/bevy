use crate::tonemapping::{TonemappingLuts, TonemappingPipeline, ViewTonemappingPipeline};

use bevy_ecs::prelude::*;
use bevy_render::{
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
#[derive(Default)]
pub struct TonemappingBindGroupCache {
    cached: Option<(BufferId, TextureViewId, TextureViewId, BindGroup)>,
    last_tonemapping: Option<Tonemapping>,
}

pub fn tonemapping(
    view: ViewQuery<(
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
    let (view_uniform_offset, target, view_tonemapping_pipeline, tonemapping) = view.into_inner();

    if *tonemapping == Tonemapping::None {
        return;
    }

    if !target.is_hdr() {
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

    let tonemapping_changed = cache
        .last_tonemapping
        .map_or(true, |last| *tonemapping != last);
    if tonemapping_changed {
        cache.last_tonemapping = Some(*tonemapping);
    }

    let bind_group = match &mut cache.cached {
        Some((buffer_id, texture_id, lut_id, bind_group))
            if view_uniforms_id == *buffer_id
                && source.id() == *texture_id
                && *lut_id != fallback_image.d3.texture_view.id()
                && !tonemapping_changed =>
        {
            bind_group
        }
        cached => {
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

            let (_, _, _, bind_group) = cached.insert((
                view_uniforms_id,
                source.id(),
                lut_bindings.0.id(),
                bind_group,
            ));
            bind_group
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

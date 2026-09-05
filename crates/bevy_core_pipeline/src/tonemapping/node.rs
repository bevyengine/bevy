use crate::tonemapping::{TonemappingLuts, TonemappingPipeline, ViewTonemappingPipeline};

use bevy_ecs::{entity::EntityHashMap, prelude::*};
use bevy_render::{
    diagnostic::RecordDiagnostics,
    render_asset::RenderAssets,
    render_resource::{
        BindGroup, BindGroupEntries, BufferId, LoadOp, Operations, PipelineCache,
        RenderPassColorAttachment, RenderPassDescriptor, StoreOp, TextureFormat, TextureViewId,
    },
    renderer::{RenderContext, ViewQuery},
    texture::{FallbackImage, GpuImage},
    view::{ViewTarget, ViewUniformOffset, ViewUniforms},
};

use super::{get_lut_bindings, Tonemapping};

/// The inputs a view's cached tonemapping bind group was created from. The
/// `camera_driver` system runs the pass once per view and each view has its
/// own post-process source, so the cache keys by view.
pub struct CachedBindGroup {
    view_uniforms: BufferId,
    source: TextureViewId,
    lut: TextureViewId,
    tonemapping: Tonemapping,
    bind_group: BindGroup,
}

pub fn tonemapping(
    view: ViewQuery<(
        Entity,
        &ViewUniformOffset,
        &ViewTarget,
        &ViewTonemappingPipeline,
    )>,
    pipeline_cache: Res<PipelineCache>,
    tonemapping_pipeline: Res<TonemappingPipeline>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    fallback_image: Res<FallbackImage>,
    view_uniforms: Res<ViewUniforms>,
    tonemapping_luts: Res<TonemappingLuts>,
    mut cache: Local<EntityHashMap<CachedBindGroup>>,
    mut ctx: RenderContext,
) {
    let (view_entity, view_uniform_offset, target, view_tonemapping_pipeline) = view.into_inner();

    // Views that run this pass always have an fp16 main texture.
    debug_assert!(!matches!(
        target.main_texture_format(),
        TextureFormat::Rgba8UnormSrgb | TextureFormat::Rgba8Unorm
    ));

    let Some(pipeline) = pipeline_cache.get_render_pipeline(view_tonemapping_pipeline.pipeline_id)
    else {
        return;
    };

    let view_uniforms_buffer = &view_uniforms.uniforms;
    let view_uniforms_id = view_uniforms_buffer.buffer().unwrap().id();

    let post_process = target.post_process_write();
    let source = post_process.source;
    let destination = post_process.destination;

    let tonemapping = view_tonemapping_pipeline.method;
    let valid = cache.get(&view_entity).is_some_and(|cached| {
        view_uniforms_id == cached.view_uniforms
            && source.id() == cached.source
            && cached.lut != fallback_image.d3.texture_view.id()
            && cached.tonemapping == tonemapping
    });
    if !valid {
        let lut_bindings = get_lut_bindings(
            &gpu_images,
            &tonemapping_luts,
            &tonemapping,
            &fallback_image,
        );

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

        cache.insert(
            view_entity,
            CachedBindGroup {
                view_uniforms: view_uniforms_id,
                source: source.id(),
                lut: lut_bindings.0.id(),
                tonemapping,
                bind_group,
            },
        );
    }
    let bind_group = &cache.get(&view_entity).unwrap().bind_group;

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

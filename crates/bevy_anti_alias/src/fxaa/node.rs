use crate::fxaa::{CameraFxaaPipeline, Fxaa, FxaaPipeline};
use bevy_ecs::prelude::*;
use bevy_render::{
    diagnostic::RecordDiagnostics,
    render_resource::{
        BindGroup, BindGroupEntries, Operations, PipelineCache, RenderPassColorAttachment,
        RenderPassDescriptor, TextureViewId,
    },
    renderer::{RenderContext, ViewQuery},
    view::ViewTarget,
};
use core::num::NonZeroU32;

pub fn fxaa(
    view: ViewQuery<(&ViewTarget, &CameraFxaaPipeline, &Fxaa)>,
    fxaa_pipeline: Res<FxaaPipeline>,
    pipeline_cache: Res<PipelineCache>,
    mut ctx: RenderContext,
    mut cached_bind_group: Local<Option<(TextureViewId, BindGroup)>>,
) {
    let (target, pipeline, fxaa) = view.into_inner();

    if !fxaa.enabled {
        return;
    }

    let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline.pipeline_id) else {
        return;
    };

    let post_process = target.post_process_write();
    let source = post_process.source;
    let destination = post_process.destination;
    let layout = if target.multiview_count().is_some() {
        &fxaa_pipeline.texture_bind_group_multiview
    } else {
        &fxaa_pipeline.texture_bind_group
    };
    let bind_group = match &mut *cached_bind_group {
        Some((id, bind_group)) if source.id() == *id => bind_group,
        cached => {
            let bind_group = ctx.render_device().create_bind_group(
                None,
                &pipeline_cache.get_bind_group_layout(layout),
                &BindGroupEntries::sequential((source, &fxaa_pipeline.sampler)),
            );

            let (_, bind_group) = cached.insert((source.id(), bind_group));
            bind_group
        }
    };

    // Broadcast across every eye layer in a single pass. The matching
    // pipeline descriptor in `mod.rs` sets the same mask. The mask is
    // `(1 << view_count) - 1` (one bit per eye); computed via
    // `u32::MAX >> (32 - view_count)` to avoid the shift overflow that
    // `1 << 32` would hit at the `MAX_VIEW_COUNT` cap.
    let view_count = target.multiview_count().map_or(1, |n| n.get());
    let multiview_mask = if view_count > 1 {
        NonZeroU32::new(u32::MAX >> (32 - view_count))
    } else {
        None
    };

    let pass_descriptor = RenderPassDescriptor {
        label: Some("fxaa"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: destination,
            depth_slice: None,
            resolve_target: None,
            ops: Operations::default(),
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask,
    };

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let time_span = diagnostics.time_span(ctx.command_encoder(), "fxaa");

    {
        let mut render_pass = ctx.command_encoder().begin_render_pass(&pass_descriptor);
        let pass_span = diagnostics.pass_span(&mut render_pass, "fxaa");

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.draw(0..3, 0..1);

        pass_span.end(&mut render_pass);
    }

    time_span.end(ctx.command_encoder());
}

use crate::tonemapping::{TonemappingBindGroups, ViewTonemappingPipeline};

use bevy_ecs::prelude::*;
use bevy_render::{
    diagnostic::RecordDiagnostics,
    render_resource::{
        LoadOp, Operations, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor, StoreOp,
    },
    renderer::{RenderContext, ViewQuery},
    view::{ViewTarget, ViewUniformOffset},
};

use super::Tonemapping;

pub fn tonemapping(
    view: ViewQuery<(
        &ViewUniformOffset,
        &ViewTarget,
        &ViewTonemappingPipeline,
        &Tonemapping,
        &TonemappingBindGroups,
    )>,
    pipeline_cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let (view_uniform_offset, target, view_tonemapping_pipeline, tonemapping, bind_groups) =
        view.into_inner();

    if *tonemapping == Tonemapping::None {
        return;
    }

    if !target.is_hdr() {
        return;
    }

    let Some(pipeline) = pipeline_cache.get_render_pipeline(view_tonemapping_pipeline.0) else {
        return;
    };

    let post_process = target.post_process_write();
    let source = post_process.source;
    let destination = post_process.destination;

    let (_, bind_group) = if bind_groups.a.0.source_id == source.id() {
        &bind_groups.a
    } else {
        &bind_groups.b
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

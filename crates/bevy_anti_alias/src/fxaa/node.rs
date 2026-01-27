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

pub(crate) fn fxaa(
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
    let bind_group = match &mut *cached_bind_group {
        Some((id, bind_group)) if source.id() == *id => bind_group,
        cached => {
            let bind_group = ctx.render_device().create_bind_group(
                None,
                &pipeline_cache.get_bind_group_layout(&fxaa_pipeline.texture_bind_group),
                &BindGroupEntries::sequential((source, &fxaa_pipeline.sampler)),
            );

            let (_, bind_group) = cached.insert((source.id(), bind_group));
            bind_group
        }
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
        multiview_mask: None,
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

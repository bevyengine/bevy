use crate::contrast_adaptive_sharpening::ViewCasPipeline;
use bevy_ecs::prelude::*;
use bevy_render::{
    diagnostic::RecordDiagnostics,
    extract_component::{ComponentUniforms, DynamicUniformIndex},
    render_resource::{
        BindGroup, BindGroupEntries, BufferId, Operations, PipelineCache,
        RenderPassColorAttachment, RenderPassDescriptor, TextureViewId,
    },
    renderer::{RenderContext, ViewQuery},
    view::{ExtractedView, ViewTarget},
};

use super::{CasPipeline, CasUniform};

pub(crate) fn cas(
    view: ViewQuery<
        (
            &ViewTarget,
            &ViewCasPipeline,
            &DynamicUniformIndex<CasUniform>,
        ),
        With<ExtractedView>,
    >,
    sharpening_pipeline: Res<CasPipeline>,
    pipeline_cache: Res<PipelineCache>,
    uniforms: Res<ComponentUniforms<CasUniform>>,
    mut ctx: RenderContext,
    mut cached_bind_group: Local<Option<(BufferId, TextureViewId, BindGroup)>>,
) {
    let (target, pipeline, uniform_index) = view.into_inner();

    let uniforms_id = uniforms.buffer().unwrap().id();
    let Some(uniforms_binding) = uniforms.binding() else {
        return;
    };

    let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline.0) else {
        return;
    };

    let view_target = target.post_process_write();
    let source = view_target.source;
    let destination = view_target.destination;

    let bind_group = match &mut *cached_bind_group {
        Some((buffer_id, texture_id, bind_group))
            if source.id() == *texture_id && uniforms_id == *buffer_id =>
        {
            bind_group
        }
        cached => {
            let bind_group = ctx.render_device().create_bind_group(
                "cas_bind_group",
                &pipeline_cache.get_bind_group_layout(&sharpening_pipeline.texture_bind_group),
                &BindGroupEntries::sequential((
                    view_target.source,
                    &sharpening_pipeline.sampler,
                    uniforms_binding,
                )),
            );

            let (_, _, bind_group) = cached.insert((uniforms_id, source.id(), bind_group));
            bind_group
        }
    };

    let pass_descriptor = RenderPassDescriptor {
        label: Some("contrast_adaptive_sharpening"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: destination,
            depth_slice: None,
            resolve_target: None,
            ops: Operations::default(),
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    };

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let time_span = diagnostics.time_span(ctx.command_encoder(), "contrast_adaptive_sharpening");

    {
        let mut render_pass = ctx.command_encoder().begin_render_pass(&pass_descriptor);

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, bind_group, &[uniform_index.index()]);
        render_pass.draw(0..3, 0..1);
    }

    time_span.end(ctx.command_encoder());
}

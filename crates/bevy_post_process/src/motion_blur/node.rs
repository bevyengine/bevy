use bevy_ecs::prelude::*;
use bevy_render::{
    diagnostic::RecordDiagnostics,
    extract_component::ComponentUniforms,
    globals::GlobalsBuffer,
    render_resource::{
        BindGroupEntries, Operations, PipelineCache, RenderPassColorAttachment,
        RenderPassDescriptor,
    },
    renderer::{RenderContext, ViewQuery},
    view::{Msaa, ViewTarget},
};

use bevy_core_pipeline::prepass::ViewPrepassTextures;

use super::{
    pipeline::{MotionBlurPipeline, MotionBlurPipelineId},
    MotionBlurUniform,
};

pub fn motion_blur(
    view: ViewQuery<(
        &ViewTarget,
        &MotionBlurPipelineId,
        &ViewPrepassTextures,
        &MotionBlurUniform,
        &Msaa,
    )>,
    motion_blur_pipeline: Res<MotionBlurPipeline>,
    pipeline_cache: Res<PipelineCache>,
    settings_uniforms: Res<ComponentUniforms<MotionBlurUniform>>,
    globals_buffer: Res<GlobalsBuffer>,
    mut ctx: RenderContext,
) {
    let (view_target, pipeline_id, prepass_textures, motion_blur_uniform, msaa) = view.into_inner();

    if motion_blur_uniform.samples == 0 || motion_blur_uniform.shutter_angle <= 0.0 {
        return; // We can skip running motion blur in these cases.
    }

    let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_id.0) else {
        return;
    };

    let Some(settings_binding) = settings_uniforms.uniforms().binding() else {
        return;
    };
    let (Some(prepass_motion_vectors_texture), Some(prepass_depth_texture)) =
        (&prepass_textures.motion_vectors, &prepass_textures.depth)
    else {
        return;
    };
    let Some(globals_uniforms) = globals_buffer.buffer.binding() else {
        return;
    };

    let post_process = view_target.post_process_write();

    let layout = if msaa.samples() == 1 {
        &motion_blur_pipeline.layout
    } else {
        &motion_blur_pipeline.layout_msaa
    };

    let bind_group = ctx.render_device().create_bind_group(
        Some("motion_blur_bind_group"),
        &pipeline_cache.get_bind_group_layout(layout),
        &BindGroupEntries::sequential((
            post_process.source,
            &prepass_motion_vectors_texture.texture.default_view,
            &prepass_depth_texture.texture.default_view,
            &motion_blur_pipeline.sampler,
            settings_binding.clone(),
            globals_uniforms.clone(),
        )),
    );

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();

    let mut render_pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("motion_blur"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: post_process.destination,
            depth_slice: None,
            resolve_target: None,
            ops: Operations::default(),
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    let pass_span = diagnostics.pass_span(&mut render_pass, "motion_blur");

    render_pass.set_render_pipeline(pipeline);
    render_pass.set_bind_group(0, &bind_group, &[]);
    render_pass.draw(0..3, 0..1);

    pass_span.end(&mut render_pass);
}

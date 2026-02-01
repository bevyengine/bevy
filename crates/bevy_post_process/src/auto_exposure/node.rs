use super::{
    buffers::AutoExposureBuffers,
    compensation_curve::GpuAutoExposureCompensationCurve,
    pipeline::{AutoExposurePipeline, ViewAutoExposurePipeline},
    AutoExposureResources,
};
use bevy_ecs::prelude::*;
use bevy_render::{
    diagnostic::RecordDiagnostics,
    globals::GlobalsBuffer,
    render_asset::RenderAssets,
    render_resource::*,
    renderer::{RenderContext, ViewQuery},
    texture::{FallbackImage, GpuImage},
    view::{ExtractedView, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
};

pub(crate) fn auto_exposure(
    view: ViewQuery<(
        &ViewUniformOffset,
        &ViewTarget,
        &ViewAutoExposurePipeline,
        &ExtractedView,
    )>,
    pipeline_cache: Res<PipelineCache>,
    pipeline: Res<AutoExposurePipeline>,
    resources: Res<AutoExposureResources>,
    view_uniforms: Res<ViewUniforms>,
    globals_buffer: Res<GlobalsBuffer>,
    auto_exposure_buffers: Res<AutoExposureBuffers>,
    fallback: Res<FallbackImage>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    compensation_curves: Res<RenderAssets<GpuAutoExposureCompensationCurve>>,
    mut ctx: RenderContext,
) {
    let view_entity = view.entity();
    let (view_uniform_offset, view_target, auto_exposure_pipeline, extracted_view) =
        view.into_inner();

    let Some(auto_exposure_buffer) = auto_exposure_buffers.buffers.get(&view_entity) else {
        return;
    };

    let (Some(histogram_pipeline), Some(average_pipeline)) = (
        pipeline_cache.get_compute_pipeline(auto_exposure_pipeline.histogram_pipeline),
        pipeline_cache.get_compute_pipeline(auto_exposure_pipeline.mean_luminance_pipeline),
    ) else {
        return;
    };

    let view_uniforms_buffer = view_uniforms.uniforms.buffer().unwrap();
    let source = view_target.main_texture_view();

    let mask = gpu_images
        .get(&auto_exposure_pipeline.metering_mask)
        .map(|i| &i.texture_view)
        .unwrap_or(&fallback.d2.texture_view);

    let Some(compensation_curve) =
        compensation_curves.get(&auto_exposure_pipeline.compensation_curve)
    else {
        return;
    };

    let compute_bind_group = ctx.render_device().create_bind_group(
        None,
        &pipeline_cache.get_bind_group_layout(&pipeline.histogram_layout),
        &BindGroupEntries::sequential((
            &globals_buffer.buffer,
            &auto_exposure_buffer.settings,
            source,
            mask,
            &compensation_curve.texture_view,
            &compensation_curve.extents,
            resources.histogram.as_entire_buffer_binding(),
            &auto_exposure_buffer.state,
            BufferBinding {
                buffer: view_uniforms_buffer,
                size: Some(ViewUniform::min_size()),
                offset: 0,
            },
        )),
    );

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let time_span = diagnostics.time_span(ctx.command_encoder(), "auto_exposure");

    {
        let mut compute_pass = ctx
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor {
                label: Some("auto_exposure"),
                timestamp_writes: None,
            });

        compute_pass.set_bind_group(0, &compute_bind_group, &[view_uniform_offset.offset]);
        compute_pass.set_pipeline(histogram_pipeline);
        compute_pass.dispatch_workgroups(
            extracted_view.viewport.z.div_ceil(16),
            extracted_view.viewport.w.div_ceil(16),
            1,
        );
        compute_pass.set_pipeline(average_pipeline);
        compute_pass.dispatch_workgroups(1, 1, 1);
    }

    time_span.end(ctx.command_encoder());
}

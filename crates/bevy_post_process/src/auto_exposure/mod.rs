use bevy_app::prelude::*;
use bevy_asset::{embedded_asset, AssetApp, Assets, Handle};
use bevy_ecs::prelude::*;
use bevy_render::{
    diagnostic::RecordDiagnostics,
    extract_component::ExtractComponentPlugin,
    render_asset::{RenderAssetPlugin, RenderAssets},
    render_resource::{
        BindGroupEntries, Buffer, BufferBinding, BufferDescriptor, BufferUsages,
        ComputePassDescriptor, PipelineCache, ShaderType, SpecializedComputePipelines,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery},
    texture::{FallbackImage, GpuImage},
    view::{ExtractedView, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
    ExtractSchedule, GpuResourceAppExt, Render, RenderApp, RenderStartup, RenderSystems,
};

mod buffers;
mod compensation_curve;
mod pipeline;
mod readback;
mod settings;

use buffers::{
    apply_pre_exposure, extract_auto_exposure, prepare_buffers, AutoExposureBuffers,
    ExtractedAutoExposure,
};
pub use compensation_curve::{AutoExposureCompensationCurve, AutoExposureCompensationCurveError};
use pipeline::{
    AutoExposurePass, AutoExposurePipeline, AutoExposureUniform, ViewAutoExposurePipeline,
};
pub use settings::{AutoExposure, AutoExposureState};

use crate::auto_exposure::{
    compensation_curve::GpuAutoExposureCompensationCurve, pipeline::init_auto_exposure_pipeline,
};
use bevy_core_pipeline::{
    schedule::{Core3d, Core3dSystems},
    tonemapping::tonemapping,
};

/// Plugin for the auto exposure feature.
///
/// See [`AutoExposure`] for more details.
pub struct AutoExposurePlugin;

#[derive(Resource)]
struct AutoExposureResources {
    histogram: Buffer,
}

impl Plugin for AutoExposurePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "auto_exposure.wesl");

        app.add_plugins(RenderAssetPlugin::<GpuAutoExposureCompensationCurve>::default())
            .init_asset::<AutoExposureCompensationCurve>()
            .register_asset_reflect::<AutoExposureCompensationCurve>();
        app.world_mut()
            .resource_mut::<Assets<AutoExposureCompensationCurve>>()
            .insert(&Handle::default(), AutoExposureCompensationCurve::default())
            .unwrap();

        app.add_plugins(ExtractComponentPlugin::<AutoExposure>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_gpu_resource::<SpecializedComputePipelines<AutoExposurePipeline>>()
            .init_resource::<AutoExposureBuffers>()
            .init_resource::<ExtractedAutoExposure>()
            .add_systems(
                RenderStartup,
                (init_auto_exposure_pipeline, init_auto_exposure_resources),
            )
            .add_systems(ExtractSchedule, extract_auto_exposure)
            .add_systems(
                Render,
                (
                    apply_pre_exposure.in_set(RenderSystems::CreateViews),
                    prepare_buffers.in_set(RenderSystems::Prepare),
                    queue_view_auto_exposure_pipelines.in_set(RenderSystems::Queue),
                ),
            )
            .add_systems(
                Core3d,
                auto_exposure
                    .before(tonemapping)
                    .in_set(Core3dSystems::PostProcess),
            );
    }
}

pub fn init_auto_exposure_resources(mut commands: Commands, render_device: Res<RenderDevice>) {
    commands.insert_resource(AutoExposureResources {
        histogram: render_device.create_buffer(&BufferDescriptor {
            label: Some("histogram buffer"),
            size: pipeline::HISTOGRAM_BIN_COUNT * 4,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        }),
    });
}

fn queue_view_auto_exposure_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut compute_pipelines: ResMut<SpecializedComputePipelines<AutoExposurePipeline>>,
    pipeline: Res<AutoExposurePipeline>,
    view_targets: Query<(Entity, &AutoExposure)>,
) {
    for (entity, auto_exposure) in view_targets.iter() {
        let histogram_pipeline =
            compute_pipelines.specialize(&pipeline_cache, &pipeline, AutoExposurePass::Histogram);
        let average_pipeline =
            compute_pipelines.specialize(&pipeline_cache, &pipeline, AutoExposurePass::Average);

        commands.entity(entity).insert(ViewAutoExposurePipeline {
            histogram_pipeline,
            mean_luminance_pipeline: average_pipeline,
            compensation_curve: auto_exposure.compensation_curve.clone(),
            metering_mask: auto_exposure.metering_mask.clone(),
        });
    }
}

/// Real time after which auto exposure starts adapting even if pipelines are still compiling.
const STARTUP_TIMEOUT_SECS: f32 = 2.0;

fn auto_exposure(
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
    auto_exposure_buffers: Res<AutoExposureBuffers>,
    render_queue: Res<RenderQueue>,
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

    // Draws whose pipelines are still compiling are skipped, so a frame rendered while any are
    // waiting doesn't start adaptation. The pipeline queue is processed just before rendering, so
    // this is current. Overwrites `pipelines_ready`, the last field of the uniform uploaded in
    // `prepare_buffers`.
    let pipelines_ready = pipeline_cache.waiting_pipelines().next().is_none()
        || auto_exposure_buffer.started.elapsed().as_secs_f32() > STARTUP_TIMEOUT_SECS;
    if let Some(settings) = auto_exposure_buffer.settings.buffer() {
        let offset = AutoExposureUniform::min_size().get() - 4;
        render_queue.write_buffer(settings, offset, &u32::from(pipelines_ready).to_le_bytes());
    }

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
            &auto_exposure_buffer.exposure,
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

    if let Some(state) = auto_exposure_buffer.state.buffer() {
        auto_exposure_buffer
            .readback
            .copy(ctx.command_encoder(), state);
    }
}

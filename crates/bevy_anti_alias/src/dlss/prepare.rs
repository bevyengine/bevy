use super::{Dlss, DlssFeature, DlssSdk};
use bevy_camera::{Camera3d, CameraMainTextureUsages, MainPassResolutionOverride};
use bevy_core_pipeline::prepass::{DepthPrepass, MotionVectorPrepass};
use bevy_diagnostic::FrameCount;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    system::{Commands, Query, Res},
};
use bevy_math::Vec4Swizzles;
use bevy_render::{
    camera::{MipBias, TemporalJitter},
    render_resource::TextureUsages,
    renderer::{RenderDevice, RenderQueue},
    view::ExtractedView,
};
use dlss_wgpu::{DlssFeatureFlags, DlssPerfQualityMode};
use std::sync::{Arc, Mutex};

#[derive(Component)]
pub struct DlssRenderContext<F: DlssFeature> {
    pub context: Mutex<F::Context>,
    pub perf_quality_mode: DlssPerfQualityMode,
    pub feature_flags: DlssFeatureFlags,
}

pub fn prepare_dlss<F: DlssFeature>(
    mut query: Query<
        (
            Entity,
            &ExtractedView,
            &Dlss<F>,
            &mut Camera3d,
            &mut CameraMainTextureUsages,
            &mut TemporalJitter,
            &mut MipBias,
            Option<&mut DlssRenderContext<F>>,
        ),
        (
            With<Camera3d>,
            With<TemporalJitter>,
            With<DepthPrepass>,
            With<MotionVectorPrepass>,
        ),
    >,
    dlss_sdk: Res<DlssSdk>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    frame_count: Res<FrameCount>,
    mut commands: Commands,
) {
    for (
        entity,
        view,
        dlss,
        mut camera_3d,
        mut camera_main_texture_usages,
        mut temporal_jitter,
        mut mip_bias,
        mut dlss_context,
    ) in &mut query
    {
        camera_main_texture_usages.0 |= TextureUsages::STORAGE_BINDING;

        let mut depth_texture_usages = TextureUsages::from(camera_3d.depth_texture_usages);
        depth_texture_usages |= TextureUsages::TEXTURE_BINDING;
        camera_3d.depth_texture_usages = depth_texture_usages.into();

        let upscaled_resolution = view.viewport.zw();

        let dlss_feature_flags = DlssFeatureFlags::LowResolutionMotionVectors
            | DlssFeatureFlags::InvertedDepth
            | DlssFeatureFlags::HighDynamicRange
            | DlssFeatureFlags::AutoExposure; // TODO

        match dlss_context.as_deref_mut() {
            Some(dlss_context)
                if upscaled_resolution
                    == F::upscaled_resolution(&dlss_context.context.lock().unwrap())
                    && dlss.perf_quality_mode == dlss_context.perf_quality_mode
                    && dlss_feature_flags == dlss_context.feature_flags =>
            {
                let dlss_context = dlss_context.context.lock().unwrap();
                let render_resolution = F::render_resolution(&dlss_context);
                temporal_jitter.offset =
                    F::suggested_jitter(&dlss_context, frame_count.0, render_resolution);
            }
            _ => {
                let dlss_context = F::new_context(
                    upscaled_resolution,
                    dlss.perf_quality_mode,
                    dlss_feature_flags,
                    Arc::clone(&dlss_sdk.0),
                    &render_device,
                    &render_queue,
                )
                .expect("Failed to create DlssRenderContext");

                let render_resolution = F::render_resolution(&dlss_context);
                temporal_jitter.offset =
                    F::suggested_jitter(&dlss_context, frame_count.0, render_resolution);
                mip_bias.0 = F::suggested_mip_bias(&dlss_context, render_resolution);

                commands.entity(entity).insert((
                    DlssRenderContext::<F> {
                        context: Mutex::new(dlss_context),
                        perf_quality_mode: dlss.perf_quality_mode,
                        feature_flags: dlss_feature_flags,
                    },
                    MainPassResolutionOverride(render_resolution),
                ));
            }
        }
    }
}

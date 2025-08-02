use super::{Dlss, DlssSdk};
use bevy_core_pipeline::{
    core_3d::Camera3d,
    prepass::{DepthPrepass, MotionVectorPrepass},
};
use bevy_diagnostic::FrameCount;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    system::{Commands, Query, Res},
};
use bevy_math::Vec4Swizzles;
use bevy_render::{
    camera::{CameraMainTextureUsages, MainPassResolutionOverride, MipBias, TemporalJitter},
    render_resource::TextureUsages,
    renderer::{RenderDevice, RenderQueue},
    view::ExtractedView,
};
use dlss_wgpu::{super_resolution::DlssSuperResolution, DlssFeatureFlags, DlssPerfQualityMode};
use std::sync::{Arc, Mutex};

#[derive(Component)]
pub struct ViewDlssSuperResolution {
    pub context: Mutex<DlssSuperResolution>,
    pub perf_quality_mode: DlssPerfQualityMode,
    pub feature_flags: DlssFeatureFlags,
}

pub fn prepare_dlss(
    mut query: Query<
        (
            Entity,
            &ExtractedView,
            &Dlss,
            &mut Camera3d,
            &mut CameraMainTextureUsages,
            &mut TemporalJitter,
            &mut MipBias,
            Option<&mut ViewDlssSuperResolution>,
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
                    == dlss_context.context.lock().unwrap().upscaled_resolution()
                    && dlss.perf_quality_mode == dlss_context.perf_quality_mode
                    && dlss_feature_flags == dlss_context.feature_flags =>
            {
                let dlss_context = dlss_context.context.lock().unwrap();
                temporal_jitter.offset =
                    dlss_context.suggested_jitter(frame_count.0, dlss_context.render_resolution());
            }
            _ => {
                let dlss_context = DlssSuperResolution::new(
                    upscaled_resolution,
                    dlss.perf_quality_mode,
                    dlss_feature_flags,
                    Arc::clone(&dlss_sdk.0),
                    render_device.wgpu_device(),
                    &render_queue,
                )
                .expect("Failed to create DlssSuperResolution");

                let render_resolution = dlss_context.render_resolution();
                temporal_jitter.offset =
                    dlss_context.suggested_jitter(frame_count.0, render_resolution);
                mip_bias.0 = dlss_context.suggested_mip_bias(render_resolution);

                commands.entity(entity).insert((
                    ViewDlssSuperResolution {
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

//! Apple MetalFX temporal denoised scaling (ray reconstruction).
//!
//! Requires macOS 26+ Apple Silicon with MetalFX Temporal Denoised Scaler support.
//!
//! See https://github.com/bevyengine/metalfx_wgpu for setup instructions.
//!
//! # Usage
//! 1. Enable Bevy's `metal_fx` feature
//! 2. Check for `Option<Res<MetalFxTemporalDenoisedScalerSupported>>` at runtime
//! 3. Add [`MetalFx`]`<`[`MetalFxTemporalDenoisedScalerFeature`]`>` to your camera

mod extract;
mod node;
mod prepare;

pub use metalfx_wgpu::MetalFxQualityMode;

use bevy_app::{App, Plugin};
use bevy_camera::Hdr;
use bevy_core_pipeline::{
    prepass::{DepthPrepass, MotionVectorPrepass, MOTION_VECTOR_PREPASS_FORMAT},
    schedule::{Core3d, Core3dSystems},
};
use bevy_ecs::prelude::*;
use bevy_math::{UVec2, Vec2};
use bevy_reflect::{reflect_remote, Reflect};
use bevy_render::{
    camera::{MipBias, TemporalJitter},
    render_resource::TextureFormat,
    renderer::RenderDevice,
    view::prepare_view_targets,
    ExtractSchedule, Render, RenderApp, RenderSystems,
};
use metalfx_wgpu::{
    query_feature_support, FeatureSupport, MetalFxFeatureFlags, MetalFxTemporalDenoisedScaler,
    MetalFxTemporalDenoisedScalerFormats, MetalFxTextureUsages,
};
use std::marker::PhantomData;
use tracing::info;

/// Enables MetalFX support once the render device exists.
#[derive(Default)]
pub struct MetalFxPlugin;

impl Plugin for MetalFxPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<MetalFx<MetalFxTemporalDenoisedScalerFeature>>();
    }

    fn finish(&self, app: &mut App) {
        let supported = {
            let render_world = app.sub_app(RenderApp).world();
            let render_device = render_world.resource::<RenderDevice>().wgpu_device();
            let mut feature_support = FeatureSupport::default();
            query_feature_support(render_device, &mut feature_support);
            feature_support.temporal_denoised_scaler_supported
        };

        if !supported {
            info!("MetalFX Temporal Denoised Scaler is not supported on this system");
            return;
        }

        app.insert_resource(MetalFxTemporalDenoisedScalerSupported);

        app.sub_app_mut(RenderApp)
            .add_systems(
                ExtractSchedule,
                extract::extract_metal_fx::<MetalFxTemporalDenoisedScalerFeature>,
            )
            .add_systems(
                Render,
                prepare::prepare_metal_fx::<MetalFxTemporalDenoisedScalerFeature>
                    .in_set(RenderSystems::PrepareViews)
                    .before(prepare_view_targets),
            )
            .add_systems(
                Core3d,
                node::metal_fx_temporal_denoised_scaler.in_set(Core3dSystems::EarlyPostProcess),
            );
    }
}

/// Camera component to enable MetalFX.
#[derive(Component, Reflect, Clone)]
#[reflect(Component)]
#[require(TemporalJitter, MipBias, DepthPrepass, MotionVectorPrepass, Hdr)]
pub struct MetalFx<F: MetalFxFeature = MetalFxTemporalDenoisedScalerFeature> {
    /// How much upscaling should be applied.
    #[reflect(remote = MetalFxQualityModeRemoteReflect)]
    pub quality_mode: MetalFxQualityMode,
    /// Set to true to delete the saved temporal history (past frames).
    pub reset: bool,
    #[reflect(ignore)]
    pub _phantom_data: PhantomData<F>,
}

impl Default for MetalFx<MetalFxTemporalDenoisedScalerFeature> {
    fn default() -> Self {
        Self {
            quality_mode: Default::default(),
            reset: Default::default(),
            _phantom_data: Default::default(),
        }
    }
}

pub trait MetalFxFeature: Reflect + Clone + Default {
    type Context: Send;

    fn upscaled_resolution(context: &Self::Context) -> UVec2;

    fn render_resolution(context: &Self::Context) -> UVec2;

    fn suggested_jitter(
        context: &Self::Context,
        frame_number: u32,
        render_resolution: UVec2,
    ) -> Vec2;

    fn suggested_mip_bias(context: &Self::Context, render_resolution: UVec2) -> f32;

    fn texture_usages(context: &Self::Context) -> MetalFxTextureUsages;

    fn new_context(
        upscaled_resolution: UVec2,
        quality_mode: MetalFxQualityMode,
        device: &RenderDevice,
    ) -> Result<Self::Context, metalfx_wgpu::MetalFxError>;
}

/// MetalFX Temporal Denoised Scaler (ray reconstruction).
///
/// Only available when the [`MetalFxTemporalDenoisedScalerSupported`] resource exists.
#[derive(Reflect, Clone, Default)]
pub struct MetalFxTemporalDenoisedScalerFeature;

impl MetalFxFeature for MetalFxTemporalDenoisedScalerFeature {
    type Context = MetalFxTemporalDenoisedScaler;

    fn upscaled_resolution(context: &Self::Context) -> UVec2 {
        context.upscaled_resolution().into()
    }

    fn render_resolution(context: &Self::Context) -> UVec2 {
        context.render_resolution().into()
    }

    fn suggested_jitter(
        context: &Self::Context,
        frame_number: u32,
        render_resolution: UVec2,
    ) -> Vec2 {
        context
            .suggested_jitter(frame_number, render_resolution.to_array())
            .into()
    }

    fn suggested_mip_bias(context: &Self::Context, render_resolution: UVec2) -> f32 {
        context.suggested_mip_bias(render_resolution.to_array())
    }

    fn texture_usages(context: &Self::Context) -> MetalFxTextureUsages {
        context.texture_usages()
    }

    fn new_context(
        upscaled_resolution: UVec2,
        quality_mode: MetalFxQualityMode,
        device: &RenderDevice,
    ) -> Result<Self::Context, metalfx_wgpu::MetalFxError> {
        let render_resolution = quality_mode.render_resolution(upscaled_resolution.to_array());
        let formats = MetalFxTemporalDenoisedScalerFormats {
            color: TextureFormat::Rgba16Float,
            // Solari's guide depth, including virtual depth for mirrors.
            depth: TextureFormat::R32Float,
            motion_vectors: MOTION_VECTOR_PREPASS_FORMAT,
            diffuse_albedo: TextureFormat::Rgba8Unorm,
            specular_albedo: TextureFormat::Rgba8Unorm,
            normals: TextureFormat::Rgba16Float,
            roughness: TextureFormat::R16Float,
            output: TextureFormat::Rgba16Float,
            specular_hit_distance: None,
            reactive_mask: None,
            denoise_strength_mask: None,
            transparency_overlay: None,
        };

        MetalFxTemporalDenoisedScaler::new(
            render_resolution,
            upscaled_resolution.to_array(),
            formats,
            MetalFxFeatureFlags::AUTO_EXPOSURE | MetalFxFeatureFlags::DEPTH_REVERSED,
            device.wgpu_device(),
        )
    }
}

#[reflect_remote(MetalFxQualityMode)]
#[derive(Default)]
enum MetalFxQualityModeRemoteReflect {
    #[default]
    Auto,
    Native,
    Quality,
    Balanced,
    Performance,
}

/// When MetalFX Temporal Denoised Scaler is supported, this resource exists in the main world.
#[derive(Resource, Clone, Copy)]
pub struct MetalFxTemporalDenoisedScalerSupported;

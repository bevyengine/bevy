use crate::{
    core_3d::graph::Core3d,
    tonemapping::{DebandDither, Tonemapping},
};
use bevy_ecs::prelude::*;
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use bevy_render::{
    camera::{Camera, CameraMainTextureUsages, CameraRenderGraph, Exposure, Projection},
    extract_component::ExtractComponent,
    primitives::Frustum,
    render_resource::{LoadOp, TextureUsages},
    view::{ColorGrading, VisibleEntities},
};
use bevy_transform::prelude::{GlobalTransform, Transform};
use serde::{Deserialize, Serialize};

/// Configuration for the "main 3d render graph".
/// The camera coordinate space is right-handed x-right, y-up, z-back.
/// This means "forward" is -Z.
#[derive(Component, Reflect, Clone, ExtractComponent)]
#[extract_component_filter(With<Camera>)]
#[reflect(Component)]
pub struct Camera3d {
    /// The depth clear operation to perform for the main 3d pass.
    pub depth_load_op: Camera3dDepthLoadOp,
    /// The texture usages for the depth texture created for the main 3d pass.
    pub depth_texture_usages: Camera3dDepthTextureUsage,
    /// How many individual steps should be performed in the [`Transmissive3d`](crate::core_3d::Transmissive3d) pass.
    ///
    /// Roughly corresponds to how many “layers of transparency” are rendered for screen space
    /// specular transmissive objects. Each step requires making one additional
    /// texture copy, so it's recommended to keep this number to a resonably low value. Defaults to `1`.
    ///
    /// ### Notes
    ///
    /// - No copies will be performed if there are no transmissive materials currently being rendered,
    ///   regardless of this setting.
    /// - Setting this to `0` disables the screen-space refraction effect entirely, and falls
    ///   back to refracting only the environment map light's texture.
    /// - If set to more than `0`, any opaque [`clear_color`](Camera::clear_color) will obscure the environment
    ///   map light's texture, preventing it from being visible “through” transmissive materials. If you'd like
    ///   to still have the environment map show up in your refractions, you can set the clear color's alpha to `0.0`.
    ///   Keep in mind that depending on the platform and your window settings, this may cause the window to become
    ///   transparent.
    pub screen_space_specular_transmission_steps: usize,
    /// The quality of the screen space specular transmission blur effect, applied to whatever's “behind” transmissive
    /// objects when their `roughness` is greater than `0.0`.
    ///
    /// Higher qualities are more GPU-intensive.
    ///
    /// **Note:** You can get better-looking results at any quality level by enabling TAA. See: [`TemporalAntiAliasPlugin`](crate::experimental::taa::TemporalAntiAliasPlugin).
    pub screen_space_specular_transmission_quality: ScreenSpaceTransmissionQuality,
}

impl Default for Camera3d {
    fn default() -> Self {
        Self {
            depth_load_op: Default::default(),
            depth_texture_usages: TextureUsages::RENDER_ATTACHMENT.into(),
            screen_space_specular_transmission_steps: 1,
            screen_space_specular_transmission_quality: Default::default(),
        }
    }
}

#[derive(Clone, Copy, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
pub struct Camera3dDepthTextureUsage(pub u32);

impl From<TextureUsages> for Camera3dDepthTextureUsage {
    fn from(value: TextureUsages) -> Self {
        Self(value.bits())
    }
}
impl From<Camera3dDepthTextureUsage> for TextureUsages {
    fn from(value: Camera3dDepthTextureUsage) -> Self {
        Self::from_bits_truncate(value.0)
    }
}

/// The depth clear operation to perform for the main 3d pass.
#[derive(Reflect, Serialize, Deserialize, Clone, Debug)]
#[reflect(Serialize, Deserialize)]
pub enum Camera3dDepthLoadOp {
    /// Clear with a specified value.
    /// Note that 0.0 is the far plane due to bevy's use of reverse-z projections.
    Clear(f32),
    /// Load from memory.
    Load,
}

impl Default for Camera3dDepthLoadOp {
    fn default() -> Self {
        Camera3dDepthLoadOp::Clear(0.0)
    }
}

impl From<Camera3dDepthLoadOp> for LoadOp<f32> {
    fn from(config: Camera3dDepthLoadOp) -> Self {
        match config {
            Camera3dDepthLoadOp::Clear(x) => LoadOp::Clear(x),
            Camera3dDepthLoadOp::Load => LoadOp::Load,
        }
    }
}

/// The quality of the screen space transmission blur effect, applied to whatever's “behind” transmissive
/// objects when their `roughness` is greater than `0.0`.
///
/// Higher qualities are more GPU-intensive.
///
/// **Note:** You can get better-looking results at any quality level by enabling TAA. See: [`TemporalAntiAliasPlugin`](crate::experimental::taa::TemporalAntiAliasPlugin).
#[derive(Resource, Default, Clone, Copy, Reflect, PartialEq, PartialOrd, Debug)]
#[reflect(Resource)]
pub enum ScreenSpaceTransmissionQuality {
    /// Best performance at the cost of quality. Suitable for lower end GPUs. (e.g. Mobile)
    ///
    /// `num_taps` = 4
    Low,

    /// A balanced option between quality and performance.
    ///
    /// `num_taps` = 8
    #[default]
    Medium,

    /// Better quality. Suitable for high end GPUs. (e.g. Desktop)
    ///
    /// `num_taps` = 16
    High,

    /// The highest quality, suitable for non-realtime rendering. (e.g. Pre-rendered cinematics and photo mode)
    ///
    /// `num_taps` = 32
    Ultra,
}

/// The camera coordinate space is right-handed x-right, y-up, z-back.
/// This means "forward" is -Z.
#[derive(Bundle, Clone)]
pub struct Camera3dBundle {
    pub camera: Camera,
    pub camera_render_graph: CameraRenderGraph,
    pub projection: Projection,
    pub visible_entities: VisibleEntities,
    pub frustum: Frustum,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub camera_3d: Camera3d,
    pub tonemapping: Tonemapping,
    pub deband_dither: DebandDither,
    pub color_grading: ColorGrading,
    pub exposure: Exposure,
    pub main_texture_usages: CameraMainTextureUsages,
}

// NOTE: ideally Perspective and Orthographic defaults can share the same impl, but sadly it breaks rust's type inference
impl Default for Camera3dBundle {
    fn default() -> Self {
        Self {
            camera_render_graph: CameraRenderGraph::new(Core3d),
            camera: Default::default(),
            projection: Default::default(),
            visible_entities: Default::default(),
            frustum: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            camera_3d: Default::default(),
            tonemapping: Default::default(),
            color_grading: Default::default(),
            exposure: Default::default(),
            main_texture_usages: Default::default(),
            deband_dither: DebandDither::Enabled,
        }
    }
}

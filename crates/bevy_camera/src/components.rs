use crate::{primitives::Frustum, Camera, CameraProjection, OrthographicProjection, Projection};
use bevy_ecs::prelude::*;
use bevy_reflect::{std_traits::ReflectDefault, Reflect, ReflectDeserialize, ReflectSerialize};
use bevy_transform::prelude::{GlobalTransform, Transform};
use serde::{Deserialize, Serialize};
use wgpu_types::TextureUsages;

/// A 2D camera component. Enables the 2D render graph for a [`Camera`].
#[derive(Component, Default, Reflect, Clone)]
#[reflect(Component, Default, Clone)]
#[require(
    Camera,
    Projection::Orthographic(OrthographicProjection::default_2d()),
    Frustum = OrthographicProjection::default_2d().compute_frustum(&GlobalTransform::from(Transform::default())),
    CameraDepthTexture
)]
pub struct Camera2d;

/// A 3D camera component. Enables the main 3D render graph for a [`Camera`].
///
/// The camera coordinate space is right-handed X-right, Y-up, Z-back.
/// This means "forward" is -Z.
#[derive(Component, Reflect, Clone, Default)]
#[reflect(Component, Default, Clone)]
#[require(Camera, Projection, CameraDepthTexture)]
pub struct Camera3d;

#[derive(Component, Reflect, Clone, Serialize, Deserialize, Debug)]
#[reflect(Component, Default, Serialize, Deserialize, Clone, Debug)]
pub struct CameraDepthTexture {
    /// The depth clear operation to perform for the main pass.
    pub load_op: CameraDepthLoadOp,
    /// The texture usages for the depth texture created for the main pass.
    pub texture_usages: CameraDepthTextureUsage,
}

impl Default for CameraDepthTexture {
    fn default() -> Self {
        Self {
            load_op: Default::default(),
            texture_usages: TextureUsages::RENDER_ATTACHMENT.into(),
        }
    }
}

#[derive(Clone, Copy, Reflect, Serialize, Deserialize, Debug)]
#[reflect(Serialize, Deserialize, Clone, Debug)]
pub struct CameraDepthTextureUsage(pub u32);

impl From<TextureUsages> for CameraDepthTextureUsage {
    fn from(value: TextureUsages) -> Self {
        Self(value.bits())
    }
}

impl From<CameraDepthTextureUsage> for TextureUsages {
    fn from(value: CameraDepthTextureUsage) -> Self {
        Self::from_bits_truncate(value.0)
    }
}

/// The depth clear operation to perform for the main pass.
#[derive(Reflect, Serialize, Deserialize, Clone, Debug)]
#[reflect(Serialize, Deserialize, Clone, Default, Debug)]
pub enum CameraDepthLoadOp {
    /// Clear with a specified value.
    /// Note that 0.0 is the far plane due to bevy's use of reverse-z projections.
    Clear(f32),
    /// Load from memory.
    Load,
}

impl Default for CameraDepthLoadOp {
    fn default() -> Self {
        CameraDepthLoadOp::Clear(0.0)
    }
}

/// If this component is added to a camera, the camera will use an intermediate "high dynamic range" render texture.
/// This allows rendering with a wider range of lighting values. However, this does *not* affect
/// whether the camera will render with hdr display output (which bevy does not support currently)
/// and only affects the intermediate render texture.
#[derive(Component, Default, Copy, Clone, Reflect, PartialEq, Eq, Hash, Debug)]
#[reflect(Component, Default, PartialEq, Hash, Debug)]
pub struct Hdr;

/// Moves a camera's tonemapping from its material shader to a separate tonemapping pass.
///
/// This adds an `Rgba16Float` main texture and one fullscreen pass. That is generally
/// cheap on desktop, but expensive on mobile. In return, the pass also tonemaps gizmos
/// and custom materials that don't tonemap themselves, and effects that read the main
/// texture before tonemapping get values that aren't tonemapped yet. Depth of field and
/// motion blur require this component. Cameras that share a render target ignore it.
#[derive(Component, Default, Copy, Clone, Reflect, PartialEq, Eq, Hash, Debug)]
#[reflect(Component, Default, PartialEq, Hash, Debug)]
pub struct TonemappingPass;

/// Color space for alpha compositing. Affects how overlapping semi-transparent layers blend.
#[derive(Component, Copy, Clone, Reflect, PartialEq, Eq, Hash, Debug, Default)]
#[reflect(Component, PartialEq, Hash, Debug, Default)]
pub enum CompositingSpace {
    /// Gamma-encoded blending. Matches most image editors. Uses default sRGB target.
    #[default]
    Srgb,
    /// Linear light blending. Physically correct.
    Linear,
    /// Perceptually uniform blending. Often smoother gradients. Requires [`Hdr`] because its value can be outside [0, 1].
    Oklab,
}

impl CompositingSpace {
    /// Whether this is the linear space, which needs no encode step.
    #[inline]
    pub fn is_linear(self) -> bool {
        matches!(self, CompositingSpace::Linear)
    }
}

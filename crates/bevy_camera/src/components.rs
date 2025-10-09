use crate::{primitives::Frustum, Camera, CameraProjection, OrthographicProjection, Projection};
use bevy_ecs::prelude::*;
use bevy_reflect::{std_traits::ReflectDefault, Reflect, ReflectDeserialize, ReflectSerialize};
use bevy_transform::prelude::{GlobalTransform, Transform};
use serde::{Deserialize, Serialize};
use wgpu_types::{LoadOp, TextureFormat, TextureUsages};

/// A 2D camera component. Enables the 2D render graph for a [`Camera`].
#[derive(Component, Default, Reflect, Clone)]
#[reflect(Component, Default, Clone)]
#[require(
    Camera,
    Projection::Orthographic(OrthographicProjection::default_2d()),
    Frustum = OrthographicProjection::default_2d().compute_frustum(&GlobalTransform::from(Transform::default())),
)]
pub struct Camera2d;

/// A 3D camera component. Enables the main 3D render graph for a [`Camera`].
///
/// The camera coordinate space is right-handed X-right, Y-up, Z-back.
/// This means "forward" is -Z.
#[derive(Component, Reflect, Clone)]
#[reflect(Component, Default, Clone)]
#[require(Camera, Projection)]
pub struct Camera3d {
    /// The depth clear operation to perform for the main 3d pass.
    pub depth_load_op: Camera3dDepthLoadOp,
    /// The texture usages for the depth texture created for the main 3d pass.
    pub depth_texture_usages: Camera3dDepthTextureUsage,
    /// How many individual steps should be performed in the `Transmissive3d` pass.
    ///
    /// Roughly corresponds to how many “layers of transparency” are rendered for screen space
    /// specular transmissive objects. Each step requires making one additional
    /// texture copy, so it's recommended to keep this number to a reasonably low value. Defaults to `1`.
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
    /// **Note:** You can get better-looking results at any quality level by enabling TAA. See: `TemporalAntiAliasPlugin`
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
#[reflect(Serialize, Deserialize, Clone)]
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
#[reflect(Serialize, Deserialize, Clone, Default)]
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
/// **Note:** You can get better-looking results at any quality level by enabling TAA. See: `TemporalAntiAliasPlugin`
#[derive(Resource, Default, Clone, Copy, Reflect, PartialEq, PartialOrd, Debug)]
#[reflect(Resource, Default, Clone, Debug, PartialEq)]
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

/// This component lets you control the [`TextureFormat`] of the DepthStencilTexture generated for the camera
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct DepthStencilFormat(TextureFormat);

impl DepthStencilFormat {
    /// Stencil format with 8 bit integer stencil.
	pub const STENCIL8 : Self = Self(TextureFormat::Stencil8);
    /// Special depth format with 16 bit integer depth.
	pub const DEPTH16_UNORM : Self = Self(TextureFormat::Depth16Unorm);
    /// Special depth format with at least 24 bit integer depth.
	pub const DEPTH24_PLUS : Self = Self(TextureFormat::Depth24Plus);
    /// Special depth/stencil format with at least 24 bit integer depth and 8 bits integer stencil.
	pub const DEPTH24_PLUS_STENCIL8 : Self = Self(TextureFormat::Depth24PlusStencil8);
    /// Special depth format with 32 bit floating point depth.
	pub const DEPTH32_FLOAT : Self = Self(TextureFormat::Depth32Float);
    /// Special depth/stencil format with 32 bit floating point depth and 8 bits integer stencil.
    ///
    /// wgpu support for [`Features::DEPTH32FLOAT_STENCIL8`] should be checked before using this format.
	pub const DEPTH32_FLOAT_STENCIL8 : Self = Self(TextureFormat::Depth32FloatStencil8);

	pub fn format(&self) -> TextureFormat {
		self.0
	}

	pub fn new(format: TextureFormat) -> Self {
		Self(format)
	}

	pub fn supports_stencil(&self) -> bool {
		matches!(
			self.0,
			TextureFormat::Depth24PlusStencil8 | TextureFormat::Depth32FloatStencil8
		)
	}
}

impl Default for DepthStencilFormat {
	fn default() -> Self {
		Self(TextureFormat::Depth32Float)
	}
}

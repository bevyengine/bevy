use bevy_ecs::{prelude::*, system::lifetimeless::Read};
use bevy_reflect::Reflect;
use bevy_render::{pipeline_keys::{PipelineKey, SystemKey, KeyShaderDefs}, render_resource::ShaderDefVal};

use crate::core_3d::Camera3d;

/// The quality of the screen space transmission blur effect, applied to whatever's “behind” transmissive
/// objects when their `roughness` is greater than `0.0`.
///
/// Higher qualities are more GPU-intensive.
///
/// **Note:** You can get better-looking results at any quality level by enabling TAA. See: [`TemporalAntiAliasPlugin`](crate::experimental::taa::TemporalAntiAliasPlugin).
#[derive(Resource, Default, Clone, Copy, Reflect, PartialEq, PartialOrd, Debug, PipelineKey)]
#[reflect(Resource)]
#[repr(u8)]
#[custom_shader_defs]
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


impl SystemKey for ScreenSpaceTransmissionQuality {
    type Param = ();

    type Query = Option<Read<Camera3d>>;

    fn from_params(_: &(), maybe_camera: Option<&Camera3d>) -> Option<Self>
    where
        Self: Sized,
    {
        maybe_camera.map(|camera| match camera.screen_space_specular_transmission_quality {
            ScreenSpaceTransmissionQuality::Low => Self::Low,
            ScreenSpaceTransmissionQuality::Medium => Self::Medium,
            ScreenSpaceTransmissionQuality::High => Self::High,
            ScreenSpaceTransmissionQuality::Ultra => Self::Ultra,
        })
    }
}

impl KeyShaderDefs for ScreenSpaceTransmissionQuality {
    fn shader_defs(&self) -> Vec<ShaderDefVal> {
        let taps = match self {
            ScreenSpaceTransmissionQuality::Low => 4,
            ScreenSpaceTransmissionQuality::Medium => 8,
            ScreenSpaceTransmissionQuality::High => 16,
            ScreenSpaceTransmissionQuality::Ultra => 32,
        };
        vec![ShaderDefVal::UInt(
            "SCREEN_SPACE_SPECULAR_TRANSMISSION_BLUR_TAPS".into(),
            taps,
        )]
    }
}

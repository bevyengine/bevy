use super::*;

/// A light that emits light in a given direction from a central point.
/// Behaves like a point light in a perfectly absorbent housing that
/// shines light only in a given direction. The direction is taken from
/// the transform, and can be specified with [`Transform::looking_at`](Transform::looking_at).
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, Default)]
pub struct SpotLight {
    pub color: Color,
    /// Luminous power in lumens, representing the amount of light emitted by this source in all directions.
    pub intensity: f32,
    pub range: f32,
    pub radius: f32,
    pub shadows_enabled: bool,
    pub shadow_depth_bias: f32,
    /// A bias applied along the direction of the fragment's surface normal. It is scaled to the
    /// shadow map's texel size so that it can be small close to the camera and gets larger further
    /// away.
    pub shadow_normal_bias: f32,
    /// Angle defining the distance from the spot light direction to the outer limit
    /// of the light's cone of effect.
    /// `outer_angle` should be < `PI / 2.0`.
    /// `PI / 2.0` defines a hemispherical spot light, but shadows become very blocky as the angle
    /// approaches this limit.
    pub outer_angle: f32,
    /// Angle defining the distance from the spot light direction to the inner limit
    /// of the light's cone of effect.
    /// Light is attenuated from `inner_angle` to `outer_angle` to give a smooth falloff.
    /// `inner_angle` should be <= `outer_angle`
    pub inner_angle: f32,
}

impl SpotLight {
    pub const DEFAULT_SHADOW_DEPTH_BIAS: f32 = 0.02;
    pub const DEFAULT_SHADOW_NORMAL_BIAS: f32 = 1.8;
}

impl Default for SpotLight {
    fn default() -> Self {
        // a quarter arc attenuating from the center
        Self {
            color: Color::WHITE,
            // 1,000,000 lumens is a very large "cinema light" capable of registering brightly at Bevy's
            // default "very overcast day" exposure level. For "indoor lighting" with a lower exposure,
            // this would be way too bright.
            intensity: 1_000_000.0,
            range: 20.0,
            radius: 0.0,
            shadows_enabled: false,
            shadow_depth_bias: Self::DEFAULT_SHADOW_DEPTH_BIAS,
            shadow_normal_bias: Self::DEFAULT_SHADOW_NORMAL_BIAS,
            inner_angle: 0.0,
            outer_angle: std::f32::consts::FRAC_PI_4,
        }
    }
}

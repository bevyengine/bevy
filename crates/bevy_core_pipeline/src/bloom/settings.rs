use bevy_ecs::prelude::Component;
use bevy_reflect::Reflect;

/// Applies a bloom effect to a HDR-enabled 2d or 3d camera.
///
/// Bloom emulates an effect found in real cameras and the human eye,
/// causing halos to appear around very bright parts of the scene.
///
/// Often used in conjunction with `bevy_pbr::StandardMaterial::emissive`.
///
/// Bloom is best used alongside a tonemapping function that desaturates bright colors,
/// such as ACES Filmic (Bevy's default).
///
/// See also <https://en.wikipedia.org/wiki/Bloom_(shader_effect)>.
#[derive(Component, Reflect, Clone)]
pub struct BloomSettings {
    pub intensity: f32,
    pub lf_boost: f32,
    pub lf_boost_curvature: f32,
    pub high_pass_frequency: f32,

    pub prefilter_settings: BloomPrefilterSettings,

    /// Compositing mode. Conthols whether bloom textures
    /// are blended between or added to each other. Useful
    /// if image brightening is desired and extremely
    /// helpful if threshold is used.
    ///
    /// # Recommendation
    /// Set to Additive if prefilter_settings is
    /// configured in a non-energy-conserving way,
    /// otherwise set to EnergyConserving.
    pub composite_mode: BloomCompositeMode,
}

impl BloomSettings {
    /// Recommended for HDR rendering
    pub const NATURAL: Self = Self {
        intensity: 0.3,
        lf_boost: 0.7,
        lf_boost_curvature: 0.95,
        high_pass_frequency: 1.0,
        prefilter_settings: BloomPrefilterSettings {
            threshold: 0.0,
            threshold_softness: 0.0,
        },
        composite_mode: BloomCompositeMode::EnergyConserving,
    };

    /// Recommended for SDR rendering
    pub const OLD_SCHOOL: Self = Self {
        intensity: 0.05,
        lf_boost: 0.7,
        lf_boost_curvature: 0.95,
        high_pass_frequency: 1.0,
        prefilter_settings: BloomPrefilterSettings {
            threshold: 0.6,
            threshold_softness: 0.2,
        },
        composite_mode: BloomCompositeMode::Additive,
    };

    pub const SCREEN_BLUR: Self = Self {
        intensity: 1.0,
        lf_boost: 0.0,
        lf_boost_curvature: 0.0,
        high_pass_frequency: 1.0 / 3.0,
        prefilter_settings: BloomPrefilterSettings {
            threshold: 0.0,
            threshold_softness: 0.0,
        },
        composite_mode: BloomCompositeMode::EnergyConserving,
    };
}

impl Default for BloomSettings {
    fn default() -> Self {
        Self::NATURAL
    }
}

/// Applies a threshold filter to the input image to
/// extract the brightest regions before blurring them and compositing
/// back onto the original image. These settings are useful if bloom is applied
/// to an SDR image or when emulating the 1990s-2000s game look.
///
/// # Considerations
/// * It is recommended to use this only if HDR rendering is not possible.
/// * Changing these settings creates a pshysically inaccurate image.
/// * Changing these settings makes it easy to make the final result look worse.
/// * Non-default prefilter settings should be used in conjuction with composite_mode::Additive
#[derive(Default, Clone, Reflect)]
pub struct BloomPrefilterSettings {
    /// Baseline of the quadratic threshold curve (default: 0.0).
    ///
    /// RGB values under the threshold curve will not contribute to the effect.
    pub threshold: f32,

    /// Controls how much to blend between the thresholded and non-thresholded colors (default: 0.0).
    ///
    /// 0.0 = Abrupt threshold, no blending
    /// 1.0 = Fully soft threshold
    ///
    /// Values outside of the range [0.0, 1.0] will be clamped.
    pub threshold_softness: f32,
}

#[derive(Clone, Reflect)]
pub enum BloomCompositeMode {
    EnergyConserving,
    Additive,
}

use bevy_ecs::prelude::Component;
use bevy_reflect::Reflect;

/// Applies a bloom effect to a 2d or 3d camera.
///
/// Bloom emulates an effect found in real cameras and the human eye,
/// causing halos to appear around very bright parts of the scene.
///
/// See also <https://en.wikipedia.org/wiki/Bloom_(shader_effect)>.
///
/// Often used in conjunction with [`bevy_pbr::StandardMaterial::emissive`]
/// for 3d meshes or [`bevy::Color::Hsla::lightness`] for 2d sprites.
///
/// Bloom is best used alongside a tonemapping function that desaturates bright colors,
/// such as ACES Filmic (Bevy's default).
///
/// Bevy's implementation uses a parametric curve to blend between a set of
/// blurred (lower frequency) images generated from the camera's view.
/// See <https://starlederer.github.io/bloom/> for a vizualization of the parametric curve
/// used in Bevy as well as a vislualization of the curve's respective scattering profile.
#[derive(Component, Reflect, Clone)]
pub struct BloomSettings {
    /// Controls the baseline of how much the image is scattered (default: 0.3).
    ///
    /// # In energy-conserving mode
    /// The value represents how likely the light is to scatter.
    ///
    /// The value should be clamed between 0.0 and 1.0 where:
    /// * 0.0 means no bloom
    /// * 1.0 means the light is scattered as much as possible
    ///
    /// # In additive mode
    /// The value represents how much scattered light is added to
    /// the image to create the glow effect.
    ///
    /// In this configuration:
    /// * 0.0 means no bloom
    /// * > 0.0 means a propotrionate amount of scattered light is added
    pub intensity: f32,

    /// Low frequency contribution boost.
    /// Controls how much more likely the light
    /// is to scatter completely sideways (low frequency image).
    ///
    /// Comparable to a low shelf boost on an equalizer.
    ///
    /// # In energy-conserving mode
    /// The value should be clamed between 0.0 and 1.0 where:
    /// * 0.0 means low frequency light uses base intensity for blend factor calculation
    /// * 1.0 means low frequency light contributes at full power
    ///
    /// # In additive mode
    /// The value represents how much scattered light is added to
    /// the image to create the glow effect.
    ///
    /// In this configuration:
    /// * 0.0 means no bloom
    /// * > 0.0 means a propotrionate amount of scattered light is added
    pub lf_boost: f32,

    /// Low frequency contribution boost curve.
    /// Controls the curvature of the blend factor function
    /// making frequncies next to lowest one contribute more.
    ///
    /// Somewhat comparable to the Q factor of an equalizer node.
    ///
    /// Valid range:
    /// * 0.0 - base base intensity and boosted intensity are lineraly interpolated
    /// * 1.0 - all frequencies below maximum are at boosted intensity level
    pub lf_boost_curvature: f32,

    /// Tightens how much the light scatters (default: 1.0).
    ///
    /// Valid range:
    /// * 0.0 - maximum scattering angle is 0deg (no scattering)
    /// * 1.0 - maximum scattering angle is 90deg
    pub high_pass_frequency: f32,

    pub prefilter_settings: BloomPrefilterSettings,

    /// Controls whether bloom textures
    /// are blended between or added to each other. Useful
    /// if image brightening is desired and a must-change
    /// if prefilter_settings are used.
    ///
    /// # Recommendation
    /// Set to Additive if prefilter_settings are
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
/// * Changing these settings creates a physically inaccurate image.
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

#[derive(Clone, Reflect, PartialEq, Eq, Hash, Copy)]
pub enum BloomCompositeMode {
    EnergyConserving,
    Additive,
}

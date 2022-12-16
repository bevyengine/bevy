use bevy_ecs::prelude::Component;
use bevy_reflect::Reflect;

#[derive(Clone, Reflect)]
pub enum BloomMode {
    EnergyConserving,
    Additive,
}

/// Applies a threshold filter to the input image to
/// extract the brightest regions before blurring them and compositing
/// back onto the original image. These settings are useful if bloom is applied
/// to an SDR image or when emulating the 90s-2000s game look.
///
/// # Considerations
/// * It is recommended to use this only if HDR rendering is not possible.
/// * Changing these settings creates a pshysically impossible image.
/// * Changing these settings makes it easy to make the final result look worse.
/// * Non-default prefilter settings should be used in conjuction with mode::Additive
#[derive(Default, Clone, Reflect)]
pub struct PrefilterSettings {
    /// Baseline of the quadratic threshold curve (default: 0.0).
    ///
    /// RGB values under the threshold curve will not have bloom applied.
    /// Using a threshold is not physically accurate, but may fit better with your artistic direction.
    pub threshold: f32,

    /// Controls how much to blend between the thresholded and non-thresholded colors (default: 0.0).
    ///
    /// 0.0 = Abrupt threshold, no blending
    /// 1.0 = Fully soft threshold
    ///
    /// Values outside of the range [0.0, 1.0] will be clamped.
    pub threshold_softness: f32,
}

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
    pub side_intensity: f32,
    pub mid_intensity: f32,
    pub mid_offset: f32,
    pub intensity: f32,

    pub prefilter_settings: PrefilterSettings,

    /// Compositing mode. Conthols whether bloom textures
    /// are blended between or added to each other. Useful
    /// if image brightening is desired and extremely
    /// helpful if threshold is used.
    /// 
    /// # Recommendation
    /// Set to Additive if prefilter_settings is
    /// configured in a non-energy-conserving way,
    /// otherwise set to EnergyConserving.
    pub mode: BloomMode,
}

impl BloomSettings {
    //// Recommended for HDR rendering
    pub const NATURAL: Self = Self {
        side_intensity: 0.4,
        mid_intensity: 0.9,
        mid_offset: 0.4,
        intensity: 0.3,
        prefilter_settings: PrefilterSettings {
            threshold: 0.0,
            threshold_softness: 0.0,
        },
        mode: BloomMode::EnergyConserving,
    };

    /// Recommended for SDR rendering
    pub const OLDSCHOOL: Self = Self {
        // TODO: Confirm whether the intensity values are good
        side_intensity: 0.8,
        mid_intensity: 0.9,
        mid_offset: 0.6,
        intensity: 0.8,
        prefilter_settings: PrefilterSettings {
            threshold: 0.6,
            threshold_softness: 0.2,
        },
        mode: BloomMode::Additive,
    };

    pub const SCREEN_BLUR: Self = Self {
        side_intensity: 0.0,
        mid_intensity: 1.0,
        mid_offset: 0.6,
        intensity: 1.0,
        prefilter_settings: PrefilterSettings {
            threshold: 0.0,
            threshold_softness: 0.0,
        },
        mode: BloomMode::EnergyConserving,
    };
}

impl Default for BloomSettings {
    fn default() -> Self {
        Self::NATURAL
    }
}

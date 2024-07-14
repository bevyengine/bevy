use super::downsampling_pipeline::BloomUniforms;
use bevy_ecs::{prelude::Component, query::QueryItem, reflect::ReflectComponent};
use bevy_math::{AspectRatio, URect, UVec4, Vec4};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{extract_component::ExtractComponent, prelude::Camera};

/// Applies a bloom effect to an HDR-enabled 2d or 3d camera.
///
/// Bloom emulates an effect found in real cameras and the human eye,
/// causing halos to appear around very bright parts of the scene.
///
/// See also <https://en.wikipedia.org/wiki/Bloom_(shader_effect)>.
///
/// # Usage Notes
///
/// **Bloom is currently not compatible with WebGL2.**
///
/// Often used in conjunction with `bevy_pbr::StandardMaterial::emissive` for 3d meshes.
///
/// Bloom is best used alongside a tonemapping function that desaturates bright colors,
/// such as [`crate::tonemapping::Tonemapping::TonyMcMapface`].
///
/// Bevy's implementation uses a parametric curve to blend between a set of
/// blurred (lower frequency) images generated from the camera's view.
/// See <https://starlederer.github.io/bloom/> for a visualization of the parametric curve
/// used in Bevy as well as a visualization of the curve's respective scattering profile.
#[derive(Component, Reflect, Clone)]
#[reflect(Component, Default)]
pub struct BloomSettings {
    /// Controls the baseline of how much the image is scattered (default: 0.15).
    ///
    /// This parameter should be used only to control the strength of the bloom
    /// for the scene as a whole. Increasing it too much will make the scene appear
    /// blurry and over-exposed.
    ///
    /// To make a mesh glow brighter, rather than increase the bloom intensity,
    /// you should increase the mesh's `emissive` value.
    ///
    /// # In energy-conserving mode
    /// The value represents how likely the light is to scatter.
    ///
    /// The value should be between 0.0 and 1.0 where:
    /// * 0.0 means no bloom
    /// * 1.0 means the light is scattered as much as possible
    ///
    /// # In additive mode
    /// The value represents how much scattered light is added to
    /// the image to create the glow effect.
    ///
    /// In this configuration:
    /// * 0.0 means no bloom
    /// * Greater than 0.0 means a proportionate amount of scattered light is added
    pub intensity: f32,

    /// Low frequency contribution boost.
    /// Controls how much more likely the light
    /// is to scatter completely sideways (low frequency image).
    ///
    /// Comparable to a low shelf boost on an equalizer.
    ///
    /// # In energy-conserving mode
    /// The value should be between 0.0 and 1.0 where:
    /// * 0.0 means low frequency light uses base intensity for blend factor calculation
    /// * 1.0 means low frequency light contributes at full power
    ///
    /// # In additive mode
    /// The value represents how much scattered light is added to
    /// the image to create the glow effect.
    ///
    /// In this configuration:
    /// * 0.0 means no bloom
    /// * Greater than 0.0 means a proportionate amount of scattered light is added
    pub low_frequency_boost: f32,

    /// Low frequency contribution boost curve.
    /// Controls the curvature of the blend factor function
    /// making frequencies next to the lowest ones contribute more.
    ///
    /// Somewhat comparable to the Q factor of an equalizer node.
    ///
    /// Valid range:
    /// * 0.0 - base intensity and boosted intensity are linearly interpolated
    /// * 1.0 - all frequencies below maximum are at boosted intensity level
    pub low_frequency_boost_curvature: f32,

    /// Tightens how much the light scatters (default: 1.0).
    ///
    /// Valid range:
    /// * 0.0 - maximum scattering angle is 0 degrees (no scattering)
    /// * 1.0 - maximum scattering angle is 90 degrees
    pub high_pass_frequency: f32,

    pub prefilter_settings: BloomPrefilterSettings,

    /// Controls whether bloom textures
    /// are blended between or added to each other. Useful
    /// if image brightening is desired and a must-change
    /// if `prefilter_settings` are used.
    ///
    /// # Recommendation
    /// Set to [`BloomCompositeMode::Additive`] if `prefilter_settings` are
    /// configured in a non-energy-conserving way,
    /// otherwise set to [`BloomCompositeMode::EnergyConserving`].
    pub composite_mode: BloomCompositeMode,
}

impl BloomSettings {
    /// The default bloom preset.
    ///
    /// This uses the [`EnergyConserving`](BloomCompositeMode::EnergyConserving) composite mode.
    pub const NATURAL: Self = Self {
        intensity: 0.15,
        low_frequency_boost: 0.7,
        low_frequency_boost_curvature: 0.95,
        high_pass_frequency: 1.0,
        prefilter_settings: BloomPrefilterSettings {
            threshold: 0.0,
            threshold_softness: 0.0,
        },
        composite_mode: BloomCompositeMode::EnergyConserving,
    };

    /// A preset that's similar to how older games did bloom.
    pub const OLD_SCHOOL: Self = Self {
        intensity: 0.05,
        low_frequency_boost: 0.7,
        low_frequency_boost_curvature: 0.95,
        high_pass_frequency: 1.0,
        prefilter_settings: BloomPrefilterSettings {
            threshold: 0.6,
            threshold_softness: 0.2,
        },
        composite_mode: BloomCompositeMode::Additive,
    };

    /// A preset that applies a very strong bloom, and blurs the whole screen.
    pub const SCREEN_BLUR: Self = Self {
        intensity: 1.0,
        low_frequency_boost: 0.0,
        low_frequency_boost_curvature: 0.0,
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

/// Applies a threshold filter to the input image to extract the brightest
/// regions before blurring them and compositing back onto the original image.
/// These settings are useful when emulating the 1990s-2000s game look.
///
/// # Considerations
/// * Changing these settings creates a physically inaccurate image
/// * Changing these settings makes it easy to make the final result look worse
/// * Non-default prefilter settings should be used in conjunction with [`BloomCompositeMode::Additive`]
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

#[derive(Debug, Clone, Reflect, PartialEq, Eq, Hash, Copy)]
pub enum BloomCompositeMode {
    EnergyConserving,
    Additive,
}

impl ExtractComponent for BloomSettings {
    type QueryData = (&'static Self, &'static Camera);

    type QueryFilter = ();
    type Out = (Self, BloomUniforms);

    fn extract_component((settings, camera): QueryItem<'_, Self::QueryData>) -> Option<Self::Out> {
        match (
            camera.physical_viewport_rect(),
            camera.physical_viewport_size(),
            camera.physical_target_size(),
            camera.is_active,
            camera.hdr,
        ) {
            (Some(URect { min: origin, .. }), Some(size), Some(target_size), true, true) => {
                let threshold = settings.prefilter_settings.threshold;
                let threshold_softness = settings.prefilter_settings.threshold_softness;
                let knee = threshold * threshold_softness.clamp(0.0, 1.0);

                let uniform = BloomUniforms {
                    threshold_precomputations: Vec4::new(
                        threshold,
                        threshold - knee,
                        2.0 * knee,
                        0.25 / (knee + 0.00001),
                    ),
                    viewport: UVec4::new(origin.x, origin.y, size.x, size.y).as_vec4()
                        / UVec4::new(target_size.x, target_size.y, target_size.x, target_size.y)
                            .as_vec4(),
                    aspect: AspectRatio::from_pixels(size.x, size.y).into(),
                };

                Some((settings.clone(), uniform))
            }
            _ => None,
        }
    }
}

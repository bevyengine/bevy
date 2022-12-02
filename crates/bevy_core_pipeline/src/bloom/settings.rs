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
    /// Intensity of the bloom effect (default: 0.3).
    pub intensity: f32,

    /// Baseline of the quadratic threshold curve (default: 0.0).
    ///
    /// RGB values under the threshold curve will not have bloom applied.
    /// Using a threshold is not physically accurate, but may fit better with your artistic direction.
    pub threshold: f32,

    /// Controls how much to blend between the thresholded and non-thresholded colors (default: 0.5).
    ///
    /// 0.0 = Abrupt threshold, no blending
    /// 1.0 = Fully soft threshold
    ///
    /// Values outside of the range [0.0, 1.0] will be clamped.
    pub threshold_softness: f32,
}

impl Default for BloomSettings {
    fn default() -> Self {
        Self {
            intensity: 0.3,
            threshold: 0.0,
            threshold_softness: 0.5,
        }
    }
}
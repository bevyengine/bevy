use std::ops::RangeInclusive;

use super::compensation_curve::AutoExposureCompensationCurve;
use bevy_asset::Handle;
use bevy_ecs::{prelude::Component, query::QueryItem, reflect::ReflectComponent};
use bevy_reflect::Reflect;
use bevy_render::{extract_component::ExtractComponent, texture::Image};
use bevy_utils::default;

/// Component that enables auto exposure for an HDR-enabled 2d or 3d camera.
///
/// Auto exposure adjusts the exposure of the camera automatically to
/// simulate the human eye's ability to adapt to different lighting conditions.
///
/// Bevy's implementation builds a 64 bin histogram of the scene's luminance,
/// and then adjusts the exposure based on that histogram. Using a histogram
/// allows the system to ignore outliers like shadows and specular highlights,
/// and it allows to give more weight to certain areas based on a mask.
///
/// # Usage Notes
///
/// **Auto Exposure requires compute shaders and is not compatible with WebGL2.**
///
#[derive(Component, Clone, Reflect)]
#[reflect(Component)]
pub struct AutoExposureSettings {
    /// The range of exposure values for the histogram.
    ///
    /// Pixel values below this range will be ignored, and pixel values above this range will be
    /// clamped in the sense that they will count towards the highest bin in the histogram.
    /// The default value is `-8.0..=8.0`.
    pub range: RangeInclusive<f32>,

    /// The portion of the histogram to consider when metering.
    ///
    /// By default, the darkest 10% and the brightest 10% of samples are ignored,
    /// so the default value is `0.10..=0.90`.
    pub filter: RangeInclusive<f32>,

    /// The speed at which the exposure adapts from dark to bright scenes, in F-stops per second.
    pub speed_brighten: f32,

    /// The speed at which the exposure adapts from bright to dark scenes, in F-stops per second.
    pub speed_darken: f32,

    /// The mask to apply when metering. The mask will cover the entire screen, where:
    /// * `(0.0, 0.0)` is the top-left corner,
    /// * `(1.0, 1.0)` is the bottom-right corner.
    /// Only the red channel of the texture is used.
    /// The sample at the current screen position will be used to weight the contribution
    /// of each pixel to the histogram:
    /// * 0.0 means the pixel will not contribute to the histogram,
    /// * 1.0 means the pixel will contribute fully to the histogram.
    ///
    /// The default value is a white image, so all pixels contribute equally.
    ///
    /// # Usage Notes
    ///
    /// The mask is quantized to 16 discrete levels because of limitations in the compute shader
    /// implementation.
    pub metering_mask: Handle<Image>,

    /// Exposure compensation curve to apply after metering.
    /// The default value is a flat line at 0.0.
    /// For more information, see [`AutoExposureCompensationCurve`].
    pub compensation_curve: Handle<AutoExposureCompensationCurve>,
}

impl Default for AutoExposureSettings {
    fn default() -> Self {
        Self {
            range: -8.0..=8.0,
            filter: 0.10..=0.90,
            speed_brighten: 3.0,
            speed_darken: 1.0,
            metering_mask: default(),
            compensation_curve: default(),
        }
    }
}

impl ExtractComponent for AutoExposureSettings {
    type QueryData = &'static Self;
    type QueryFilter = ();
    type Out = Self;

    fn extract_component(item: QueryItem<'_, Self::QueryData>) -> Option<Self> {
        Some(item.clone())
    }
}

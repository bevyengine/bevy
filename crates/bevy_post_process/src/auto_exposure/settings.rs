use core::ops::RangeInclusive;

use super::compensation_curve::AutoExposureCompensationCurve;
use bevy_asset::Handle;
use bevy_camera::Hdr;
use bevy_ecs::{prelude::Component, reflect::ReflectComponent};
use bevy_image::Image;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{extract_component::ExtractComponent, RenderApp};
use bevy_utils::default;

/// Component that enables auto exposure for an HDR-enabled 2d or 3d camera.
///
/// Auto exposure adjusts the exposure of the camera automatically to
/// simulate the human eye's ability to adapt to different lighting conditions.
///
/// Bevy's implementation builds a 64 bin histogram of the scene's luminance,
/// and then adjusts the exposure so that the average brightness of the final
/// render will be middle gray. Because it's using a histogram, some details can
/// be selectively ignored or emphasized. Outliers like shadows and specular
/// highlights can be ignored, and certain areas can be given more (or less)
/// weight based on a mask.
///
/// # Usage Notes
///
/// **Auto Exposure requires compute shaders and is not compatible with WebGL2.**
#[derive(Component, Clone, Reflect, ExtractComponent)]
#[reflect(Component, Default, Clone)]
#[require(Hdr)]
#[extract_app(RenderApp)]
pub struct AutoExposure {
    /// The range of exposure values for the histogram.
    ///
    /// Pixel values below this range count towards the lowest bin and are metered at the minimum
    /// luminance. Pixel values above this range count towards the highest bin in the histogram.
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

    /// The distance in F-stops from the target exposure from where to transition from animating
    /// in linear fashion to animating exponentially. This helps against jittering when the
    /// target exposure keeps on changing slightly from frame to frame, while still maintaining
    /// a relatively slow animation for big changes in scene brightness.
    ///
    /// ```text
    /// ev
    ///                       ➔●┐
    /// |              ⬈         ├ exponential section
    /// │        ⬈               ┘
    /// │    ⬈                   ┐
    /// │  ⬈                     ├ linear section
    /// │⬈                       ┘
    /// ●───────────────────────── time
    /// ```
    ///
    /// The default value is 1.5.
    pub exponential_transition_distance: f32,

    /// The mask to apply when metering. The mask will cover the entire screen, where:
    /// * `(0.0, 0.0)` is the top-left corner,
    /// * `(1.0, 1.0)` is the bottom-right corner.
    ///
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

    /// The minimum and maximum exposure adjustments that auto exposure can apply, in stops.
    ///
    /// A correction of `1.0` brightens the image by one stop, and `-1.0` darkens it by one stop.
    /// This limits the automatic correction independently of the luminance [`range`](Self::range)
    /// used for the histogram. Setting this to `0.0..=0.0` disables the automatic correction.
    ///
    /// The applied correction, including compensation from
    /// [`compensation_curve`](Self::compensation_curve), always stays within this range.
    /// It is applied in addition to the camera's [`Exposure`](bevy_camera::Exposure) and
    /// color grading settings.
    ///
    /// Neither limit can be NaN or infinite, and the minimum must not be greater than the maximum.
    /// Invalid ranges are ignored with a one-time warning.
    ///
    /// The default value is `f32::MIN..=f32::MAX`, which does not limit the correction.
    pub correction_range: RangeInclusive<f32>,
}

impl Default for AutoExposure {
    fn default() -> Self {
        Self {
            range: -8.0..=8.0,
            filter: 0.10..=0.90,
            speed_brighten: 3.0,
            speed_darken: 1.0,
            exponential_transition_distance: 1.5,
            metering_mask: default(),
            compensation_curve: default(),
            correction_range: f32::MIN..=f32::MAX,
        }
    }
}

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
/// Bevy's implementation builds a 128 bin histogram of the scene's luminance,
/// and then adjusts the exposure so that the average brightness of the final
/// render will be middle gray. Because it's using a histogram, some details can
/// be selectively ignored or emphasized. Outliers like shadows and specular
/// highlights can be ignored, and certain areas can be given more (or less)
/// weight based on a mask.
///
/// Metering uses absolute scene luminance in [EV100], with the exposure the
/// view was rendered with divided out. The adapted exposure moves gradually
/// toward the metered target over real time ([`Time<Real>`](bevy_time::Real)),
/// so it keeps adapting while virtual time is paused, and a newly added camera
/// starts at its first metered target. Tonemapping applies the difference
/// between the adapted exposure and the exposure the view was rendered with.
///
/// # Pre-exposure
///
/// With [`pre_exposure`](Self::pre_exposure) on (the default), the view renders
/// with the adapted exposure, so its HDR main texture keeps its precision for
/// scenes within the metering [`range`](Self::range). The adapted exposure is read back from the GPU, so it
/// is a few frames old, usually one to three. Until
/// the first readback arrives, and after a [`reset`](Self::reset) until the
/// next one, the view renders with the camera's [`Exposure`].
///
/// Auto exposure never writes the camera's [`Exposure`]. It is the reference
/// for [`correction_range`](Self::correction_range), and the exposure the view
/// renders with when pre-exposure is off.
///
/// [`AutoExposureState`] holds the adapted exposure in the main world. In the
/// render world, auto exposure writes the displayed exposure to the view's
/// [`ViewExposureTexture`](bevy_core_pipeline::tonemapping::ViewExposureTexture).
///
/// [EV100]: https://en.wikipedia.org/wiki/Exposure_value
/// [`Exposure`]: bevy_camera::Exposure
///
/// # Usage Notes
///
/// **Auto Exposure requires compute shaders and is not compatible with WebGL2.**
#[derive(Component, Clone, Reflect, ExtractComponent)]
#[reflect(Component, Default, Clone)]
#[require(Hdr, AutoExposureState)]
#[extract_app(RenderApp)]
pub struct AutoExposure {
    /// The range of scene luminance the histogram covers, in EV100.
    ///
    /// A luminance of `L` cd/m² is `log2(L) + 3` EV100. A mid-gray surface is about 15 EV100 in
    /// direct sunlight, 8 in an office and -3 under a full moon.
    ///
    /// Pixel values below this range count towards the lowest bin and are metered at the minimum
    /// luminance. Pixel values above this range count towards the highest bin and are metered at
    /// the maximum luminance.
    /// The default value is `-8.0..=20.0`.
    pub range: RangeInclusive<f32>,

    /// The portion of the histogram to consider when metering.
    ///
    /// By default, the darkest 10% and the brightest 10% of samples are ignored,
    /// so the default value is `0.10..=0.90`.
    pub filter: RangeInclusive<f32>,

    /// The speed at which the exposure adapts from dark to bright scenes, in F-stops per second
    /// of real time.
    pub speed_brighten: f32,

    /// The speed at which the exposure adapts from bright to dark scenes, in F-stops per second
    /// of real time.
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
    /// A compensation of `0.0` exposes the metered average to 1.0 before color grading and
    /// tonemapping; each stop of compensation doubles that.
    /// The default value is a flat line at 0.0.
    /// For more information, see [`AutoExposureCompensationCurve`].
    pub compensation_curve: Handle<AutoExposureCompensationCurve>,

    /// The minimum and maximum exposure adjustments that auto exposure can apply relative to the
    /// camera's [`Exposure`](bevy_camera::Exposure), in stops.
    ///
    /// A correction of `1.0` brightens the image by one stop, and `-1.0` darkens it by one stop.
    /// This limits the automatic correction independently of the luminance [`range`](Self::range)
    /// used for the histogram. Setting this to `0.0..=0.0` displays the camera's exposure.
    ///
    /// The applied correction, including compensation from
    /// [`compensation_curve`](Self::compensation_curve), always stays within this range, and the
    /// adapted exposure is clamped to it around the camera's exposure. The correction is applied
    /// in addition to the color grading settings.
    ///
    /// Neither limit can be NaN or infinite, and the minimum must not be greater than the maximum.
    /// Invalid ranges are ignored with a one-time warning.
    ///
    /// The default value is `f32::MIN..=f32::MAX`, which does not limit the correction.
    pub correction_range: RangeInclusive<f32>,

    /// Whether the view renders with the adapted exposure instead of the camera's
    /// [`Exposure`](bevy_camera::Exposure). See [pre-exposure](Self#pre-exposure).
    ///
    /// Turn this off when the app sets the camera's exposure itself to keep the main texture in
    /// range.
    ///
    /// The default value is `true`.
    pub pre_exposure: bool,

    /// Set to true to jump the adapted exposure to the metered target this frame instead of
    /// adapting toward it, such as at a camera cut.
    ///
    /// It is set back to false when the camera is extracted.
    pub reset: bool,
}

/// The adapted exposure of a camera with [`AutoExposure`], read back from the GPU.
///
/// Auto exposure writes this; changes made to it are overwritten. The value is a few frames old,
/// usually one to three, and holds its last value while the camera is inactive.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Component, Default, Clone, Debug, PartialEq)]
pub struct AutoExposureState {
    /// The adapted exposure, in EV100: the exposure the image is displayed with. `None` until a
    /// readback arrives, and again from a [`reset`](AutoExposure::reset) until the next one.
    pub ev100: Option<f32>,
}

impl Default for AutoExposure {
    fn default() -> Self {
        Self {
            range: -8.0..=20.0,
            filter: 0.10..=0.90,
            speed_brighten: 3.0,
            speed_darken: 1.0,
            exponential_transition_distance: 1.5,
            metering_mask: default(),
            compensation_curve: default(),
            correction_range: f32::MIN..=f32::MAX,
            pre_exposure: true,
            reset: false,
        }
    }
}

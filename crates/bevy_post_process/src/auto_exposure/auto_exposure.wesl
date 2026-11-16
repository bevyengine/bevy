//! Auto exposure
//!
//! This shader meters the scene's absolute luminance and moves an adapted exposure gradually
//! toward it. The tone mapping shader applies the difference between the adapted exposure and the
//! camera's own, so the image doesn't depend on the exposure the view was rendered with.
//!
//! The adapted exposure is computed in two passes:
//! * The compute_histogram pass calculates a histogram of the scene luminance in EV100,
//! taking into account the metering mask texture. The metering mask is a grayscale texture
//! that defines the areas of the screen that should be given more weight when calculating
//! the average luminance value. For example, the middle area of the screen might be more important
//! than the edges.
//! * The compute_average pass calculates the average luminance value of the scene, taking
//! into account the low_percent and high_percent settings. These settings define the
//! percentage of the histogram that should be excluded when calculating the average. This
//! is useful to avoid overexposure when you have a lot of shadows, or underexposure when you
//! have a lot of bright specular reflections.
//!
//! The target exposure displays that average at the compensated level, and the adapted exposure
//! moves toward it over real time.

import bevy_render::view::View;

/// Constant to convert RGB to luminance, taken from Real Time Rendering, Vol 4 pg. 278, 4th edition
const RGB_TO_LUM = vec3<f32>(0.2125, 0.7154, 0.0721);

/// The number of histogram bins; matches `HISTOGRAM_BIN_COUNT`. The first and last bins hold the
/// luminance below and above the metering range, the others split the range evenly.
const BIN_COUNT = 128u;

/// `log2(1.2)`. A camera at `ev100` stores luminance `L` as `L * 2^-ev100 / 1.2`
/// (`Exposure::exposure`).
const LOG2_EXPOSURE_SCALE = 0.26303441;

/// The EV100 of luminance `L` (cd/m²) is `log2(L) + 3`: a reflected-light meter at ISO 100 with
/// calibration constant K = 12.5, `log2(L * 100 / 12.5)`.
const EV100_OF_LOG2_LUMINANCE = 3.0;

struct AutoExposure {
    min_ev100: f32,
    inv_ev100_range: f32,
    ev100_range: f32,
    low_percent: f32,
    high_percent: f32,
    speed_brighten: f32,
    speed_darken: f32,
    exponential_transition_distance: f32,
    correction_min: f32,
    correction_max: f32,
    /// Real time since the previous frame, in seconds.
    delta_time: f32,
    /// Nonzero when no pipeline is waiting to compile, or the startup timeout has passed.
    pipelines_ready: u32,
}

struct CompensationCurve {
    min_ev100: f32,
    inv_ev100_range: f32,
    min_compensation: f32,
    compensation_range: f32,
}

/// The adapted exposure, persistent across frames.
struct AutoExposureState {
    /// The adapted exposure, in EV100.
    ev100: f32,
    /// Zero until the first fully drawn frame with luminance above the bottom of the metering
    /// range sets `ev100` to its target.
    initialized: u32,
}

@group(0) @binding(0) var<uniform> settings: AutoExposure;

@group(0) @binding(1) var tex_color: texture_2d<f32>;

@group(0) @binding(2) var tex_mask: texture_2d<f32>;

@group(0) @binding(3) var tex_compensation: texture_1d<f32>;

@group(0) @binding(4) var<uniform> compensation_curve: CompensationCurve;

@group(0) @binding(5) var<storage, read_write> histogram: array<atomic<u32>, BIN_COUNT>;

@group(0) @binding(6) var<storage, read_write> state: AutoExposureState;

@group(0) @binding(7) var<storage, read_write> view: View;

var<workgroup> histogram_shared: array<atomic<u32>, BIN_COUNT>;

/// For a given stored color, return the histogram bin index of its scene luminance.
fn color_to_bin(hdr: vec3<f32>, log2_exposure: f32) -> u32 {
    // Convert color to luminance
    let lum = dot(hdr, RGB_TO_LUM);

    // Zero, negative and NaN luminance have no logarithm; count them in the lowest bin.
    if !(lum > 0.0) {
        return 0u;
    }

    // Undo the view's exposure to get the scene luminance in EV100, and express it as a
    // position where [0.0, 1.0] spans the metering range.
    let ev100 = log2(lum) - log2_exposure + EV100_OF_LOG2_LUMINANCE;
    let position = (ev100 - settings.min_ev100) * settings.inv_ev100_range;

    // Map [0, 1) to [1, BIN_COUNT - 2], below the range to the first bin, and the top of the range
    // and above to the last bin.
    return u32(clamp(position * f32(BIN_COUNT - 2u) + 1.0, 0.0, f32(BIN_COUNT - 1u)));
}

/// The position of a histogram bin within the metering range, from 0.0 at the minimum to 1.0 at
/// the maximum. The first and last bins sit at the ends, the others at their centers.
fn bin_position(index: u32) -> f32 {
    return saturate((f32(index) - 0.5) / f32(BIN_COUNT - 2u));
}

/// Read the metering mask at the given UV coordinates, returning a weight for the histogram.
///
/// Since the histogram is summed in the compute_average step, there is a limit to the amount of
/// distinct values that can be represented. When using the chosen value of 16, the maximum
/// amount of pixels that can be weighted and summed is 2^32 / 16 = 16384^2.
fn metering_weight(coords: vec2<f32>) -> u32 {
    let pos = vec2<i32>(coords * vec2<f32>(textureDimensions(tex_mask)));
    let mask = textureLoad(tex_mask, pos, 0).r;
    return u32(mask * 16.0);
}

@compute @workgroup_size(16, 16, 1)
fn compute_histogram(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(local_invocation_index) local_invocation_index: u32
) {
    // Clear the workgroup shared histogram
    if local_invocation_index < BIN_COUNT {
        atomicStore(&histogram_shared[local_invocation_index], 0u);
    }

    // Wait for all workgroup threads to clear the shared histogram
    workgroupBarrier();

    let dim = vec2<u32>(textureDimensions(tex_color));
    let uv = vec2<f32>(global_invocation_id.xy) / vec2<f32>(dim);

    if global_invocation_id.x < dim.x && global_invocation_id.y < dim.y {
        let col = textureLoad(tex_color, vec2<i32>(global_invocation_id.xy), 0).rgb;
        let index = color_to_bin(col, log2(view.exposure));
        let weight = metering_weight(uv);

        // Increment the shared histogram bin by the weight obtained from the metering mask
        atomicAdd(&histogram_shared[index], weight);
    }

    // Wait for all workgroup threads to finish updating the workgroup histogram
    workgroupBarrier();

    // Accumulate the workgroup histogram into the global histogram, one bin per invocation.
    // Note that the global histogram was not cleared at the beginning,
    // as it will be cleared in compute_average.
    if local_invocation_index < BIN_COUNT {
        let histogram_value = atomicLoad(&histogram_shared[local_invocation_index]);
        if histogram_value != 0u {
            atomicAdd(&histogram[local_invocation_index], histogram_value);
        }
    }
}

@compute @workgroup_size(1, 1, 1)
fn compute_average() {
    // The cumulative histogram is accumulated on the fly in the averaging loop rather than stored
    // in a function-local array: NVIDIA's shader compiler (driver 621.01) asserts on that array.
    var histogram_sum = 0u;
    for (var i=0u; i<BIN_COUNT; i+=1u) {
        histogram_sum += atomicLoad(&histogram[i]);
    }

    // A frame with every pixel in the lowest bin, such as a black frame before the scene has
    // loaded, doesn't start adaptation.
    let metered = atomicLoad(&histogram[0]) < histogram_sum;

    let first_index = u32(f32(histogram_sum) * settings.low_percent);
    let last_index = u32(f32(histogram_sum) * settings.high_percent);

    var count = 0u;
    var sum = 0.0;
    // Include the lowest bin at the minimum luminance when averaging.
    // The cumulative histogram before this bin is zero, clamped to first_index.
    var previous = first_index;
    // The sum of all bins up to and including bin i.
    var running_sum = 0u;
    for (var i=0u; i<BIN_COUNT; i+=1u) {
        running_sum += atomicLoad(&histogram[i]);

        // Clear the histogram bin for the next frame
        atomicStore(&histogram[i], 0u);

        // The number of pixels in the bin. The histogram values are clamped to
        // first_index and last_index to exclude the lowest and highest samples.
        let cumulative = clamp(running_sum, first_index, last_index);
        let bin_count = cumulative - previous;
        previous = cumulative;

        sum += f32(bin_count) * bin_position(i);
        count += bin_count;
    }

    // The average scene luminance of the included histogram samples, in EV100.
    var avg_ev100 = settings.min_ev100;

    if count > 0u {
        avg_ev100 = sum / f32(count) * settings.ev100_range + settings.min_ev100;
    }

    // The position in the compensation curve texture to sample for avg_ev100.
    let u = (avg_ev100 - compensation_curve.min_ev100) * compensation_curve.inv_ev100_range;
    let compensation = textureLoad(tex_compensation, i32(saturate(u) * 255.0), 0).r
        * compensation_curve.compensation_range
        + compensation_curve.min_compensation;

    // The target exposure stores the average luminance at 2^compensation:
    // log2(L) - ev100 - log2(1.2) = compensation.
    let target_ev100 = avg_ev100 - EV100_OF_LOG2_LUMINANCE - LOG2_EXPOSURE_SCALE - compensation;

    var ev100 = target_ev100;
    if state.initialized != 0u {
        // Smoothly adjust the adapted exposure towards the target. A brighter scene raises the
        // exposure value.
        let delta = target_ev100 - state.ev100;
        let speed = select(settings.speed_darken, settings.speed_brighten, delta > 0.0)
            * settings.delta_time;
        let exponential = min(speed / settings.exponential_transition_distance, 1.0);
        ev100 = state.ev100 + clamp(delta * exponential, -speed, speed);
    }

    // Keep the correction relative to the camera's exposure within the configured range, and
    // store the clamped exposure so adaptation continues from what is displayed.
    let camera_ev100 = -log2(view.exposure) - LOG2_EXPOSURE_SCALE;
    ev100 = clamp(
        ev100,
        camera_ev100 - settings.correction_max,
        camera_ev100 - settings.correction_min,
    );
    state.ev100 = ev100;
    if metered && settings.pipelines_ready != 0u {
        state.initialized = 1u;
    }

    // Apply the correction to the color grading settings, from where it will be used for the
    // color grading pass.
    view.color_grading.exposure += camera_ev100 - ev100;
}

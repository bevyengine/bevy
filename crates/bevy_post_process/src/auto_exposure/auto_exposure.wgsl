// Auto exposure
//
// This shader computes an auto exposure value for the current frame,
// which is then used as an exposure correction in the tone mapping shader.
//
// The auto exposure value is computed in two passes:
// * The compute_histogram pass calculates a histogram of the luminance values in the scene,
// taking into account the metering mask texture. The metering mask is a grayscale texture
// that defines the areas of the screen that should be given more weight when calculating
// the average luminance value. For example, the middle area of the screen might be more important
// than the edges.
// * The compute_average pass calculates the average luminance value of the scene, taking
// into account the low_percent and high_percent settings. These settings define the
// percentage of the histogram that should be excluded when calculating the average. This
// is useful to avoid overexposure when you have a lot of shadows, or underexposure when you
// have a lot of bright specular reflections.
//
// The final target_exposure is finally used to smoothly adjust the exposure value over time.

#import bevy_render::view::View
#import bevy_render::globals::Globals

// Constant to convert RGB to luminance, taken from Real Time Rendering, Vol 4 pg. 278, 4th edition
const RGB_TO_LUM = vec3<f32>(0.2125, 0.7154, 0.0721);

struct AutoExposure {
    min_log_lum: f32,
    inv_log_lum_range: f32,
    log_lum_range: f32,
    low_percent: f32,
    high_percent: f32,
    speed_up: f32,
    speed_down: f32,
    exponential_transition_distance: f32,
}

struct CompensationCurve {
    min_log_lum: f32,
    inv_log_lum_range: f32,
    min_compensation: f32,
    compensation_range: f32,
}

@group(0) @binding(0) var<uniform> globals: Globals;

@group(0) @binding(1) var<uniform> settings: AutoExposure;

@group(0) @binding(2) var tex_color: texture_2d<f32>;

@group(0) @binding(3) var tex_mask: texture_2d<f32>;

@group(0) @binding(4) var tex_compensation: texture_1d<f32>;

@group(0) @binding(5) var<uniform> compensation_curve: CompensationCurve;

@group(0) @binding(6) var<storage, read_write> histogram: array<atomic<u32>, 64>;

@group(0) @binding(7) var<storage, read_write> exposure: f32;

@group(0) @binding(8) var<storage, read_write> view: View;

var<workgroup> histogram_shared: array<atomic<u32>, 64>;

// For a given color, return the histogram bin index
fn color_to_bin(hdr: vec3<f32>) -> u32 {
    // Convert color to luminance
    let lum = dot(hdr, RGB_TO_LUM);

    if lum < exp2(settings.min_log_lum) {
        return 0u;
    }

    // Calculate the log_2 luminance and express it as a value in [0.0, 1.0]
    // where 0.0 represents the minimum luminance, and 1.0 represents the max.
    let log_lum = saturate((log2(lum) - settings.min_log_lum) * settings.inv_log_lum_range);

    // Map [0, 1] to [1, 63]. The zeroth bin is handled by the epsilon check above.
    return u32(log_lum * 62.0 + 1.0);
}

// Read the metering mask at the given UV coordinates, returning a weight for the histogram.
//
// Since the histogram is summed in the compute_average step, there is a limit to the amount of
// distinct values that can be represented. When using the chosen value of 16, the maximum
// amount of pixels that can be weighted and summed is 2^32 / 16 = 16384^2.
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
    if local_invocation_index < 64 {
        histogram_shared[local_invocation_index] = 0u;
    }

    // Wait for all workgroup threads to clear the shared histogram
    workgroupBarrier();

    let dim = vec2<u32>(textureDimensions(tex_color));
    let uv = vec2<f32>(global_invocation_id.xy) / vec2<f32>(dim);

    if global_invocation_id.x < dim.x && global_invocation_id.y < dim.y {
        let col = textureLoad(tex_color, vec2<i32>(global_invocation_id.xy), 0).rgb;
        let index = color_to_bin(col);
        let weight = metering_weight(uv);

        // Increment the shared histogram bin by the weight obtained from the metering mask
        atomicAdd(&histogram_shared[index], weight);
    }

    // Wait for all workgroup threads to finish updating the workgroup histogram
    workgroupBarrier();

    // Accumulate the workgroup histogram into the global histogram.
    // Note that the global histogram was not cleared at the beginning,
    // as it will be cleared in compute_average.
    atomicAdd(&histogram[local_invocation_index], histogram_shared[local_invocation_index]);
}

@compute @workgroup_size(1, 1, 1)
fn compute_average(@builtin(local_invocation_index) local_index: u32) {
    var histogram_sum = 0u;

    // Calculate the cumulative histogram and clear the histogram bins.
    // Each bin in the cumulative histogram contains the sum of all bins up to that point.
    // This way we can quickly exclude the portion of lowest and highest samples as required by
    // the low_percent and high_percent settings.
    for (var i=0u; i<64u; i+=1u) {
        histogram_sum += histogram[i];
        histogram_shared[i] = histogram_sum;

        // Clear the histogram bin for the next frame
        histogram[i] = 0u;
    }

    let first_index = u32(f32(histogram_sum) * settings.low_percent);
    let last_index = u32(f32(histogram_sum) * settings.high_percent);

    var count = 0u;
    var sum = 0.0;
    for (var i=1u; i<64u; i+=1u) {
        // The number of pixels in the bin. The histogram values are clamped to
        // first_index and last_index to exclude the lowest and highest samples.
        let bin_count =
            clamp(histogram_shared[i], first_index, last_index) -
            clamp(histogram_shared[i - 1u], first_index, last_index);

        sum += f32(bin_count) * f32(i);
        count += bin_count;
    }

    var avg_lum = settings.min_log_lum;

    if count > 0u {
        // The average luminance of the included histogram samples.
        avg_lum = sum / (f32(count) * 63.0)
            * settings.log_lum_range
            + settings.min_log_lum;
    }

    // The position in the compensation curve texture to sample for avg_lum.
    let u = (avg_lum - compensation_curve.min_log_lum) * compensation_curve.inv_log_lum_range;

    // The target exposure is the negative of the average log luminance.
    // The compensation value is added to the target exposure to adjust the exposure for
    // artistic purposes.
    let target_exposure = textureLoad(tex_compensation, i32(saturate(u) * 255.0), 0).r
        * compensation_curve.compensation_range
        + compensation_curve.min_compensation
        - avg_lum;

    // Smoothly adjust the `exposure` towards the `target_exposure`
    let delta = target_exposure - exposure;
    if target_exposure > exposure {
        let speed_down = settings.speed_down * globals.delta_time;
        let exp_down = speed_down / settings.exponential_transition_distance;
        exposure = exposure + min(speed_down, delta * exp_down);
    } else {
        let speed_up = settings.speed_up * globals.delta_time;
        let exp_up = speed_up / settings.exponential_transition_distance;
        exposure = exposure + max(-speed_up, delta * exp_up);
    }

    // Apply the exposure to the color grading settings, from where it will be used for the color
    // grading pass.
    view.color_grading.exposure += exposure;
}

#import bevy_render::view::View

// Taken from RTR vol 4 pg. 278
const RGB_TO_LUM = vec3<f32>(0.2125, 0.7154, 0.0721);

struct Params {
    min_log_lum: f32,
    inv_log_lum_range: f32,
    log_lum_range: f32,
    low_percent: u32,
    high_percent: u32,
    speed_up: f32,
    speed_down: f32,
}

@group(0) @binding(0)
var<uniform> params: Params;
@group(0) @binding(1)
var tex_color: texture_2d<f32>;
@group(0) @binding(2)
var tex_mask: texture_2d<f32>;
@group(0) @binding(3)
var tex_compensation: texture_1d<f32>;
@group(0) @binding(4)
var<storage, read_write> histogram: array<atomic<u32>, 256>;
@group(0) @binding(5)
var<storage, read_write> exposure: f32;
@group(0) @binding(6)
var<storage, read_write> view: View;

var<workgroup> histogram_shared: array<atomic<u32>, 256>;

// For a given color and luminance range, return the histogram bin index
fn colorToBin(hdrColor: vec3<f32>, minLogLum: f32, inverseLogLumRange: f32) -> u32 {
    let lum = dot(hdrColor, RGB_TO_LUM);

    if lum < exp2(minLogLum) {
        return 0u;
    }

    // Calculate the log_2 luminance and express it as a value in [0.0, 1.0]
    // where 0.0 represents the minimum luminance, and 1.0 represents the max.
    let logLum = saturate((log2(lum) - minLogLum) * inverseLogLumRange);

    // Map [0, 1] to [1, 255]. The zeroth bin is handled by the epsilon check above.
    return u32(logLum * 254.0 + 1.0);
}

@compute @workgroup_size(16, 16, 1)
fn computeHistogram(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(local_invocation_index) local_invocation_index: u32
) {
    histogram_shared[local_invocation_index] = 0u;
    storageBarrier();

    let dim = vec2<u32>(textureDimensions(tex_color));
    let uv = vec2<f32>(global_invocation_id.xy) / vec2<f32>(dim);

    if global_invocation_id.x < dim.x && global_invocation_id.y < dim.y {
        let col = textureLoad(tex_color, vec2<i32>(global_invocation_id.xy), 0).rgb;
        let index = colorToBin(col, params.min_log_lum, params.inv_log_lum_range);
        let mask = textureLoad(tex_mask, vec2<i32>(uv * vec2<f32>(textureDimensions(tex_mask))), 0).r;

        atomicAdd(&histogram_shared[index], u32(mask * 8.0));
    }

    workgroupBarrier();
    atomicAdd(&histogram[local_invocation_index], histogram_shared[local_invocation_index]);
}

@compute @workgroup_size(1, 1, 1)
fn computeAverage(@builtin(local_invocation_index) local_index: u32) {
    var histogram_sum = 0u;
    for (var i=0u; i<256u; i+=1u) {
        histogram_sum += histogram[i];
        histogram_shared[i] = histogram_sum;
        histogram[i] = 0u;
    }

    let first_index = histogram_sum * params.low_percent / 100u;
    let last_index = histogram_sum * params.high_percent / 100u;

    var count = 0u;
    var sum = 0.0;
    for (var i=1u; i<256u; i+=1u) {
        let bin_count =
            clamp(histogram_shared[i], first_index, last_index) -
            clamp(histogram_shared[i - 1u], first_index, last_index);

        sum += f32(bin_count) * f32(i);
        count += bin_count;
    }

    var target_exposure = 0.0;

    if count > 0u {
        let avg_bin = sum / f32(count);
        let avg_lum = avg_bin / 255.0 * params.log_lum_range + params.min_log_lum;
        target_exposure += -8.0 + textureLoad(tex_compensation, i32(avg_bin), 0).r * 16.0 - avg_lum;
    }

    exposure = exposure + clamp(target_exposure - exposure, -params.speed_up, params.speed_down);
    view.color_grading.exposure = exposure;
}

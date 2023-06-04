#import bevy_render::view

@group(0) @binding(0)
var<storage, read_write> view: View;

@group(0) @binding(1)
var hdr_texture: texture_2d<f32>;

@group(0) @binding(2)
var<storage, read_write> global_luminance_histogram: array<atomic<u32>, 64>;

var<workgroup> luminance_histogram: array<atomic<u32>, 64>;

@compute @workgroup_size(16, 16, 1)
fn build_histogram(@builtin(global_invocation_id) global_id: vec3<u32>, @builtin(local_invocation_id) local_id: vec3<u32>) {
    let texture_size = textureDimensions(hdr_texture, 0);
    if (global_id.x < u32(view.viewport.z) && global_id.y < u32(view.viewport.w)) {
        let color = textureLoad(hdr_texture, global_id.xy + vec2<u32>(view.viewport.xy), 0);
        let luminance = color.r * 0.2126 + color.g * 0.7152 + color.b * 0.0722;
        if (luminance > 0.0) {
            let ev = log2(luminance) + 3.0;
            let bin = u32(clamp(round(ev * 2.0 + 16.0), 1.0, 63.0));
            atomicAdd(&luminance_histogram[bin], 1u);
        } else {
            atomicAdd(&luminance_histogram[0], 1u);
        }
    }

    workgroupBarrier();

    let index = local_id.x + local_id.y * 16u;
    if (index < 64u) {
        atomicAdd(&global_luminance_histogram[index], luminance_histogram[index]);
    }
}

@compute @workgroup_size(1)
fn compute_exposure() {
    var sum = 0u;
    for (var i = 0; i < 64; i++) {
        sum += global_luminance_histogram[i];
        luminance_histogram[i] = sum;
    }

    let first_index = sum * 40u / 100u;
    let last_index = sum * 95u / 100u;

    var count = 0u;
    var sum_ev = 0.0;
    for (var i = 1; i < 64; i++) {
        if (luminance_histogram[i] < first_index) {
            continue;
        }

        let bin_count = min(luminance_histogram[i], last_index)
            - min(luminance_histogram[i - 1], last_index);
        sum_ev += f32(bin_count) * (f32(i) - 16.0) * 0.5;
        count += bin_count;
   }

    var target_exposure = 0.0;
    if (count > 0u) {
        // TODO: Apply exposure compensation curve so that night scenes seem darker than day scenes.
        let avg_ev = sum_ev / f32(count);
        target_exposure = log2(1.2) - avg_ev;
    }

    // TODO: Use a moving average to smooth out the exposure changes.
    view.color_grading.exposure = target_exposure;
}

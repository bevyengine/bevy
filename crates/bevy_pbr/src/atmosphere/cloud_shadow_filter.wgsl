#define_import_path bevy_pbr::atmosphere::cloud_shadow_filter

#import bevy_pbr::atmosphere::bindings::{settings, atmosphere_lut_sampler}

// Source cloud shadow map (rgba16f sampled as float).
// Layout matches `bindings.wgsl`:
// - R: front depth (km)
// - G: mean extinction (1/m)
// - B: max optical depth (unitless)
@group(0) @binding(17) var cloud_shadow_src: texture_2d<f32>;

// Destination ping-pong texture.
@group(0) @binding(13) var cloud_shadow_dst: texture_storage_2d<rgba16float, write>;

fn load_shadow(coord: vec2<i32>) -> vec3<f32> {
    // Sample at texel centers using linear filtering (sampler clamps to edge).
    let size = vec2<f32>(settings.cloud_shadow_map_size);
    let uv = (vec2<f32>(coord) + vec2(0.5)) / size;
    return textureSampleLevel(cloud_shadow_src, atmosphere_lut_sampler, uv, 0.0).rgb;
}

// Spatial filter in transmittance space for max optical depth (Unreal-style idea):
// - Filter visibility V = exp(-OD) with a small kernel
// - Convert back: OD = -log(V)
// Keep depth mostly unfiltered to avoid precision / convergence issues.
@compute
// Keep in sync with `dispatch_2d()` in `atmosphere/node.rs` (currently assumes 16x16 workgroups).
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let size = settings.cloud_shadow_map_size;
    if (gid.x >= size.x || gid.y >= size.y) {
        return;
    }

    let p = vec2<i32>(gid.xy);
    let center = load_shadow(p);

    // 3x3 tent-ish kernel weights:
    // 1 2 1
    // 2 4 2
    // 1 2 1
    // sum = 16
    var sum_mean_ext = 0.0;
    var sum_vis = 0.0;
    var sum_w = 0.0;

    for (var dy: i32 = -1; dy <= 1; dy += 1) {
        for (var dx: i32 = -1; dx <= 1; dx += 1) {
            let w = select(1.0, 2.0, (dx == 0) || (dy == 0));
            let tap = load_shadow(p + vec2(dx, dy));
            let mean_ext = tap.y;
            let od = max(0.0, tap.z);
            let vis = exp(-od);
            sum_mean_ext += w * mean_ext;
            sum_vis += w * vis;
            sum_w += w;
        }
    }

    let mean_ext_out = sum_mean_ext / max(1.0, sum_w);
    let vis_out = sum_vis / max(1.0, sum_w);
    // Avoid -inf / NaN from log(0).
    let od_out = select(100.0, -log(max(vis_out, 1e-6)), vis_out > 0.0);

    // Depth: pass through unchanged in the transmittance blur pass.
    let out_rgb = vec3(center.x, max(0.0, mean_ext_out), max(0.0, od_out));
    textureStore(cloud_shadow_dst, p, vec4(out_rgb, 0.0));
}



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

fn sort2(a: ptr<function, f32>, b: ptr<function, f32>) {
    if ((*a) > (*b)) {
        let t = (*a);
        (*a) = (*b);
        (*b) = t;
    }
}

// Median of 9 values (3x3). Used to denoise front-depth without the “expanding rings”
// you get from iterative min/max dilate/erode passes.
fn median9(
    v0: f32, v1: f32, v2: f32,
    v3: f32, v4: f32, v5: f32,
    v6: f32, v7: f32, v8: f32
) -> f32 {
    var a = v0; var b = v1; var c = v2;
    var d = v3; var e = v4; var f = v5;
    var g = v6; var h = v7; var i = v8;

    sort2(&a, &b); sort2(&d, &e); sort2(&g, &h);
    sort2(&b, &c); sort2(&e, &f); sort2(&h, &i);
    sort2(&a, &b); sort2(&d, &e); sort2(&g, &h);

    sort2(&a, &d); sort2(&d, &g);
    sort2(&b, &e); sort2(&e, &h);
    sort2(&c, &f); sort2(&f, &i);

    sort2(&c, &e); sort2(&e, &g);
    sort2(&c, &e); sort2(&e, &g);

    return e;
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

    // Depth denoise:
    // Use a conservative median filter on valid depth samples. This reduces noisy “salt-and-pepper”
    // in the front-depth channel without the runaway growth you can get from repeated dilation.
    //
    // NOTE: This pass can run multiple iterations (ping-pong). To keep it stable over multiple
    // passes, we only blend *partially* toward the median.
    const DEPTH_INVALID: f32 = 1.0e9;
    const DEPTH_DENOISE_ALPHA: f32 = 1.0;

    let d00 = load_shadow(p + vec2(-1, -1)).x;
    let d10 = load_shadow(p + vec2( 0, -1)).x;
    let d20 = load_shadow(p + vec2( 1, -1)).x;
    let d01 = load_shadow(p + vec2(-1,  0)).x;
    let d11 = center.x;
    let d21 = load_shadow(p + vec2( 1,  0)).x;
    let d02 = load_shadow(p + vec2(-1,  1)).x;
    let d12 = load_shadow(p + vec2( 0,  1)).x;
    let d22 = load_shadow(p + vec2( 1,  1)).x;

    // Treat non-positive depths as "invalid / no cloud" so they don't dominate the median.
    let v00 = select(DEPTH_INVALID, d00, d00 > 0.0);
    let v10 = select(DEPTH_INVALID, d10, d10 > 0.0);
    let v20 = select(DEPTH_INVALID, d20, d20 > 0.0);
    let v01 = select(DEPTH_INVALID, d01, d01 > 0.0);
    let v11 = select(DEPTH_INVALID, d11, d11 > 0.0);
    let v21 = select(DEPTH_INVALID, d21, d21 > 0.0);
    let v02 = select(DEPTH_INVALID, d02, d02 > 0.0);
    let v12 = select(DEPTH_INVALID, d12, d12 > 0.0);
    let v22 = select(DEPTH_INVALID, d22, d22 > 0.0);

    let med = median9(v00, v10, v20, v01, v11, v21, v02, v12, v22);
    let depth_med = select(center.x, med, med < 1.0e8);
    let depth_out = mix(center.x, depth_med, DEPTH_DENOISE_ALPHA);

    let out_rgb = vec3(depth_out, max(0.0, mean_ext_out), max(0.0, od_out));
    textureStore(cloud_shadow_dst, p, vec4(out_rgb, 0.0));
}



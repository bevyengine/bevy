// Temporal reconstruction of the checkerboarded cloud trace (design.md §1.2, roadmap step 1d).
//
// `clouds_raymarch` traces 1 of each 2x2 half-res pixels per frame into a quarter-size trace
// texture. This pass rebuilds the full half-res cloud buffer:
// - the pixel traced this frame takes the new sample (blended with clamped history),
// - the other 3 reproject last frame's buffer using the *transmittance-weighted cloud depth*
//   (FB16 §5.9.1; RDR2 slide 52), clamped to the 3x3 (L, T) AABB of the fresh trace,
// - pixels without valid history fall back to the nearest traced ray.

import package::atmosphere::{
    bindings::{atmosphere, view, atmosphere_lut_sampler},
    functions::{uv_to_ray_direction, get_view_position},
};

struct CloudsTemporalParams {
    prev_clip_from_world: mat4x4<f32>,
    // Cloud field displacement since the previous frame (atmosphere space, wind advection).
    cloud_motion: vec3<f32>,
    history_valid: u32,
    // Weight of the new sample for the pixel traced this frame.
    new_sample_weight: f32,
}

@group(0) @binding(26) var trace_texture: texture_2d<f32>;
@group(0) @binding(27) var trace_depth_texture: texture_2d<f32>;
@group(0) @binding(28) var history_texture: texture_2d<f32>;
@group(0) @binding(29) var history_depth_texture: texture_2d<f32>;
@group(0) @binding(30) var<uniform> params: CloudsTemporalParams;
@group(0) @binding(31) var out_clouds: texture_storage_2d<rgba16float, write>;
@group(0) @binding(32) var out_clouds_depth: texture_storage_2d<r32float, write>;
@if(CLOUD_REFERENCE)
@group(0) @binding(33) var<storage, read_write> ref_accum: array<vec4<f32>>;

const CHECKER_OFFSETS: array<vec2<u32>, 4> = array(
    vec2(0u, 0u), vec2(1u, 1u), vec2(1u, 0u), vec2(0u, 1u)
);
// Distance used to reproject pixels whose rays saw no cloud (rotation-only reprojection).
// Salvi 2016 variance clipping width.
const RECON_VARIANCE_GAMMA: f32 = 1.25;
const NO_CLOUD_REPROJECTION_DISTANCE_M: f32 = 100000.0;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(out_clouds);
    if (any(id.xy >= size)) {
        return;
    }
    let trace_size = vec2<i32>(textureDimensions(trace_texture));
    let q = min(vec2<i32>(id.xy / 2u), trace_size - 1);
    let traced_now = all((id.xy % 2u) == CHECKER_OFFSETS[view.frame_count % 4u]);

    let center = textureLoad(trace_texture, q, 0);
    let center_depth = textureLoad(trace_depth_texture, q, 0).r;

    // 3x3 neighbourhood statistics of the fresh trace in (L, T): variance clipping box
    // (Salvi 2016, mean +- gamma * sigma) instead of the min/max AABB, which with a noisy trace
    // is as wide as the noise and let it through un-averaged (far / horizon clouds).
    var c_min = center;
    var c_max = center;
    var m1 = vec4(0.0);
    var m2 = vec4(0.0);
    var d_max = center_depth;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let p = clamp(q + vec2(x, y), vec2(0), trace_size - 1);
            let c = textureLoad(trace_texture, p, 0);
            c_min = min(c_min, c);
            c_max = max(c_max, c);
            m1 += c;
            m2 += c * c;
            d_max = max(d_max, textureLoad(trace_depth_texture, p, 0).r);
        }
    }

    let mean = m1 / 9.0;
    let sigma = sqrt(max(m2 / 9.0 - mean * mean, vec4(0.0)));
    c_min = max(c_min, mean - RECON_VARIANCE_GAMMA * sigma);
    c_max = min(c_max, mean + RECON_VARIANCE_GAMMA * sigma);

    // Without history (first frame / resize) the 3 untraced pixels of each 2x2 quad would copy the
    // nearest traced ray (blocky); interpolate the quarter-res trace instead.
    let full_uv = (vec2<f32>(id.xy) + 0.5) / vec2<f32>(size);
    var result = select(center, textureSampleLevel(trace_texture, atmosphere_lut_sampler, full_uv, 0.0), !traced_now);
    var result_depth = center_depth;

    if (params.history_valid != 0u) {
        // Reproject with the weighted cloud depth of this pixel's neighbourhood.
        let uv = (vec2<f32>(id.xy) + 0.5) / vec2<f32>(size);
        let ray_dir = uv_to_ray_direction(uv);
        let d = select(NO_CLOUD_REPROJECTION_DISTANCE_M, center_depth, center_depth > 0.0);
        // Cloud points moved with the wind since last frame (RDR2 uses the wind as the motion
        // vector of its volumes); clear rays reproject by rotation only.
        var motion = select(vec3(0.0), params.cloud_motion, center_depth > 0.0);
        @if(ATM_DBG_NO_CLOUD_MOTION) { motion = vec3(0.0); }
        let p_atmo = get_view_position() + ray_dir * d - motion;
        let p_world = (atmosphere.atmosphere_to_world * vec4(p_atmo, 1.0)).xyz;
        let prev_clip = params.prev_clip_from_world * vec4(p_world, 1.0);
        let prev_ndc = prev_clip.xy / prev_clip.w;
        let prev_uv = prev_ndc * vec2(0.5, -0.5) + vec2(0.5);

        if (prev_clip.w > 0.0 && all(prev_uv >= vec2(0.0)) && all(prev_uv <= vec2(1.0))) {
            let history = textureSampleLevel(history_texture, atmosphere_lut_sampler, prev_uv, 0.0);
            let prev_px = vec2<i32>(min(prev_uv * vec2<f32>(size), vec2<f32>(size) - 1.0));
            let history_depth = textureLoad(history_depth_texture, prev_px, 0).r;
            let clamped = clamp(history, c_min, c_max);
            if (traced_now) {
                result = mix(clamped, center, params.new_sample_weight);
            } else {
                result = clamped;
                // Keep the history depth unless it disagrees with every fresh neighbour.
                result_depth = select(center_depth, history_depth, history_depth <= d_max * 1.5);
            }
        }
    }

    // Reference path tracer: progressive average per pixel (each traced every 4th frame) once
    // the noise/shadow map have settled; no clamping.
    // Reference path tracer: exact running mean in an f32 accumulator (sum + count), started
    // once the noise/shadow map have settled. No clamping, no history blending.
    @if(CLOUD_REFERENCE) {
        const REF_START: u32 = 120u;
        let k = 2u * (id.y * size.x + id.x);
        if (view.frame_count < REF_START) {
            ref_accum[k] = vec4(0.0);
            ref_accum[k + 1u] = vec4(0.0);
        } else {
            var sum = ref_accum[k];
            var cnt = ref_accum[k + 1u];
            if (traced_now && all(center == center)) {
                sum += center;
                cnt += vec4(1.0, center_depth, 0.0, 0.0);
                ref_accum[k] = sum;
                ref_accum[k + 1u] = cnt;
            }
            if (cnt.x > 0.0) {
                result = sum / cnt.x;
                result_depth = cnt.y / cnt.x;
            }
        }
    }
    // NaN/Inf guard so one bad sample can't stick in the history forever.
    if (any(result != result) || any(abs(result) > vec4(65000.0))) {
        result = vec4(0.0, 0.0, 0.0, 1.0);
    }
    textureStore(out_clouds, id.xy, result);
    textureStore(out_clouds_depth, id.xy, vec4(result_depth, 0.0, 0.0, 0.0));
}

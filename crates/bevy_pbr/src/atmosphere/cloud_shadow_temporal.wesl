
// Temporal filter for cloud shadow map. Reduces edge flickering by blending
// with history. Based on Unreal's MainShadowTemporalProcessCS and
// three-geospatial's ShadowResolveMaterial.
//
// Layout matches cloud_shadow_map:
// - R: front depth (km) from light-volume near plane
// - G: mean extinction (1/m)
// - B: max optical depth (unitless)
//
// Note: We do NOT import bindings::settings here because it would conflict
// with our binding 1 (curr_cloud_shadow). Size is passed via temporal_params.

@group(0) @binding(3) var prev_cloud_sampler: sampler;

@group(0) @binding(0) var<uniform> temporal_params: CloudShadowTemporalParams;
// Traced output (storage, use textureLoad)
@group(0) @binding(1) var curr_cloud_shadow: texture_storage_2d<rgba16float, read>;
// History (sampled for bilinear)
@group(0) @binding(2) var prev_cloud_shadow: texture_2d<f32>;
@group(0) @binding(4) var out_cloud_shadow: texture_storage_2d<rgba16float, write>;

struct CloudShadowTemporalParams {
    curr_anchor: vec3<f32>,
    curr_light_dir: vec3<f32>,
    curr_basis_x: vec3<f32>,
    curr_basis_y: vec3<f32>,
    prev_anchor: vec3<f32>,
    prev_light_dir: vec3<f32>,
    prev_basis_x: vec3<f32>,
    prev_basis_y: vec3<f32>,
    extent: f32,
    temporal_alpha: f32,
    history_valid: u32,
    anchor_moved: u32,
    size: vec2<u32>,
    interleave: u32,
    frame_index: u32,
}

// Time-slicing: the texel (within each N x N tile) traced on a given frame, in Bayer order so
// that consecutive frames are spread out spatially. Must match `cloud_shadow_temporal.wesl`.
fn cloud_shadow_interleave_slot(frame: u32, n: u32) -> vec2<u32> {
    if (n >= 4u) {
        // Inverse 4x4 Bayer matrix: rank -> packed (x + 4 * y).
        const ORDER4 = array<u32, 16>(0u, 10u, 2u, 8u, 5u, 15u, 7u, 13u, 1u, 11u, 3u, 9u, 4u, 14u, 6u, 12u);
        let packed = ORDER4[frame % 16u];
        return vec2(packed % 4u, packed / 4u);
    }
    if (n == 2u) {
        // Inverse 2x2 Bayer matrix: rank -> packed (x + 2 * y).
        const ORDER2 = array<u32, 4>(0u, 3u, 1u, 2u);
        let packed = ORDER2[frame % 4u];
        return vec2(packed % 2u, packed / 2u);
    }
    return vec2(0u);
}

// Reprojects the previous frame's cloud shadow map into the current texel. Returns the history
// value in `rgb` (front depth already converted to the current near plane) and `w = 1` when the
// reprojection is in bounds.
//
// `nearest` point-samples the history. Untraced texels must use it: they are re-reprojected every
// frame, so bilinear resampling would keep blurring the map (and blend front depths of cloudy and
// clear texels into meaningless values).
//
// The reprojection happens at the occluder depth (the history's front depth at this texel), not
// at the near plane: the near plane sits `2 * extent` away from the anchor, so even the slow sun
// rotation of a day/night cycle moves it by more than a texel per frame.
fn reproject_cloud_shadow_history(coord: vec2<i32>, uv: vec2<f32>, extent: f32, half_depth: f32, nearest: bool) -> vec4<f32> {
    let max_t = 2.0 * half_depth;
    let curr_ray_origin = temporal_params.curr_anchor
        + temporal_params.curr_basis_x * ((uv.x - 0.5) * 2.0 * extent)
        + temporal_params.curr_basis_y * ((uv.y - 0.5) * 2.0 * extent)
        - temporal_params.curr_light_dir * half_depth;
    // Depth estimate: history front depth at the same texel ("no cloud" texels use the
    // anchor depth, where the visible shadows are).
    let depth_estimate_km = textureLoad(prev_cloud_shadow, coord, 0).r;
    var depth_estimate = depth_estimate_km * 1000.0;
    if (depth_estimate >= 0.999 * max_t) {
        depth_estimate = half_depth;
    }
    let world_pos = curr_ray_origin + temporal_params.curr_light_dir * depth_estimate;

    let prev_ray_origin = temporal_params.prev_anchor - temporal_params.prev_light_dir * half_depth;
    let prev_rel = world_pos - temporal_params.prev_anchor;
    let prev_uv = vec2(
        dot(prev_rel, temporal_params.prev_basis_x),
        dot(prev_rel, temporal_params.prev_basis_y),
    ) / (2.0 * extent) + vec2(0.5);
    if (any(prev_uv < vec2(0.0)) || any(prev_uv > vec2(1.0))) {
        return vec4(0.0);
    }

    var prev: vec3<f32>;
    if (nearest) {
        let size = vec2<i32>(temporal_params.size);
        let prev_coord = clamp(vec2<i32>(floor(prev_uv * vec2<f32>(size))), vec2(0), size - vec2(1));
        prev = textureLoad(prev_cloud_shadow, prev_coord, 0).rgb;
    } else {
        prev = textureSampleLevel(prev_cloud_shadow, prev_cloud_sampler, prev_uv, 0.0).rgb;
    }
    // Front depth: previous near plane -> current near plane (keeps "no cloud" at max depth).
    // Only when the anchor moved: the map is Rgba16Float, so converting every frame (e.g. for the
    // slow sun rotation) would re-quantize the depth, which is stored in km at up to 4 * extent.
    if (temporal_params.anchor_moved != 0u && prev.r * 1000.0 < 0.999 * max_t) {
        let prev_pos = prev_ray_origin
            + (temporal_params.prev_basis_x * dot(prev_rel, temporal_params.prev_basis_x)
                + temporal_params.prev_basis_y * dot(prev_rel, temporal_params.prev_basis_y))
            + temporal_params.prev_light_dir * (prev.r * 1000.0);
        prev.r = clamp(dot(prev_pos - curr_ray_origin, temporal_params.curr_light_dir), 0.0, max_t) * 0.001;
    }
    return vec4(prev, 1.0);
}

@compute
@workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let size = temporal_params.size;
    if (gid.x >= size.x || gid.y >= size.y) {
        return;
    }

    let coord = vec2<i32>(gid.xy);
    let size_f = vec2<f32>(size);
    let uv = (vec2<f32>(gid.xy) + vec2(0.5)) / size_f;
    let extent = temporal_params.extent;
    let half_depth = extent * 2.0;

    // Current frame data (traced output). With time-slicing, only one texel per N x N tile was
    // traced this frame; the others still hold the value traced on an earlier frame.
    let curr_data = textureLoad(curr_cloud_shadow, coord).rgb;
    let interleave = max(1u, temporal_params.interleave);
    let slot = cloud_shadow_interleave_slot(temporal_params.frame_index, interleave);
    let traced_this_frame = all((gid.xy % vec2(interleave)) == slot);

    var filtered_data = curr_data;

    if (temporal_params.history_valid != 0u && !traced_this_frame) {
        // Not traced this frame: reproject the previous filtered result (falls back to the
        // stale traced value when the history is out of bounds).
        let reprojected = reproject_cloud_shadow_history(coord, uv, extent, half_depth, true);
        if (reprojected.w > 0.5) {
            filtered_data = reprojected.rgb;
        }
    } else if (temporal_params.history_valid != 0u) {
        // Clamp-to-neighborhood (AABB): compute min/max of G and B in 3x3 neighborhood.
        // Prevents history from contributing outlier values at edges, reducing jitter.
        // Based on Unreal VolumetricRenderTarget and TSR/TAA.
        var g_min = curr_data.g;
        var g_max = curr_data.g;
        var b_min = curr_data.b;
        var b_max = curr_data.b;
        let size_i = vec2<i32>(size);
        for (var dy = -1; dy <= 1; dy += 1) {
            for (var dx = -1; dx <= 1; dx += 1) {
                let nc = clamp(coord + vec2<i32>(dx, dy), vec2<i32>(0, 0), size_i - vec2<i32>(1, 1));
                let n = textureLoad(curr_cloud_shadow, nc).rgb;
                g_min = min(g_min, n.g);
                g_max = max(g_max, n.g);
                b_min = min(b_min, n.b);
                b_max = max(b_max, n.b);
            }
        }

        let reprojected = reproject_cloud_shadow_history(coord, uv, extent, half_depth, false);
        if (reprojected.w > 0.5) {
            var prev = reprojected.rgb;

            // Clamp history to neighborhood AABB before blending (reduces edge jitter)
            prev.g = clamp(prev.g, g_min, g_max);
            prev.b = clamp(prev.b, b_min, b_max);

            // Temporal blend: prev + alpha * (curr - prev)
            // Do NOT filter depth (Unreal: precision/convergence issues)
            let alpha = temporal_params.temporal_alpha;
            filtered_data.g = prev.g + alpha * (curr_data.g - prev.g);
            filtered_data.b = prev.b + alpha * (curr_data.b - prev.b);
            filtered_data.r = curr_data.r;
        }
    }

    textureStore(out_cloud_shadow, coord, vec4(max(filtered_data, vec3(0.0)), 0.0));
}

#define_import_path bevy_pbr::atmosphere::cloud_shadow_temporal

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

    // Current frame data (traced output)
    let curr_data = textureLoad(curr_cloud_shadow, coord).rgb;

    var filtered_data = curr_data;

    if (temporal_params.history_valid != 0u) {
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

        // Reproject: current texel UV -> world position -> previous UV
        // World position at near plane for this texel (Unreal-style)
        let xy = (uv - vec2(0.5)) * (2.0 * extent);
        let world_pos = temporal_params.curr_anchor
            + temporal_params.curr_basis_x * xy.x
            + temporal_params.curr_basis_y * xy.y
            - temporal_params.curr_light_dir * half_depth;

        // Map world position to previous frame's UV
        let prev_rel = world_pos - temporal_params.prev_anchor;
        let prev_x = dot(prev_rel, temporal_params.prev_basis_x);
        let prev_y = dot(prev_rel, temporal_params.prev_basis_y);
        let prev_uv = vec2(prev_x, prev_y) / (2.0 * extent) + vec2(0.5);

        // Reject history when reprojection is out of bounds (three-geospatial style)
        let in_bounds = prev_uv.x >= 0.0 && prev_uv.x <= 1.0 && prev_uv.y >= 0.0 && prev_uv.y <= 1.0;
        if (in_bounds) {
            var prev = textureSampleLevel(prev_cloud_shadow, prev_cloud_sampler, prev_uv, 0.0).rgb;

            // Depth reprojection when anchor moved (Unreal-style)
            // prev.r is distance from near plane along the ray; near plane is anchor - light_dir * half_depth.
            if (temporal_params.anchor_moved != 0u) {
                let prev_depth_m = prev.r * 1000.0;
                let prev_ray_origin = temporal_params.prev_anchor - temporal_params.prev_light_dir * half_depth;
                let prev_pos = prev_ray_origin + temporal_params.prev_light_dir * prev_depth_m;
                let curr_ray_origin = temporal_params.curr_anchor - temporal_params.curr_light_dir * half_depth;
                let curr_depth_from_near = dot(prev_pos - curr_ray_origin, temporal_params.curr_light_dir);
                prev.r = curr_depth_from_near * 0.001;
            }

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

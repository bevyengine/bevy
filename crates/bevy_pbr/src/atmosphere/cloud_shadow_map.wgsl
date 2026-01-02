#define_import_path bevy_pbr::atmosphere::cloud_shadow_map

// Computes an Unreal-style cloud shadow map storing:
// R: front depth (kilometers) from the light-volume near plane (stored in km to fit fp16 range)
// G: mean extinction (1/m)
// B: max optical depth (unitless)
//
// This is later sampled at arbitrary world points to cheaply compute
// transmittance: T = exp(-tau) where tau = mean_ext * (d_sample - d_front),
// clamped to max optical depth.

#import bevy_pbr::atmosphere::bindings::{atmosphere, settings, view, lights}
#import bevy_pbr::atmosphere::clouds::{
    cloud_layer_segment,
    get_cloud_medium_density,
    get_cloud_scattering_coeff,
    get_cloud_absorption_coeff,
}

@group(0) @binding(13) var out_cloud_shadow_map: texture_storage_2d<rgba16float, write>;

const EPSILON_M: f32 = 1.0;
const CLOUD_SHADOW_DEBUG_WRITE_UV: bool = false;

fn safe_normalize(v: vec3<f32>) -> vec3<f32> {
    let l2 = dot(v, v);
    if (l2 <= 1e-12) {
        return vec3(0.0, 1.0, 0.0);
    }
    return v * inverseSqrt(l2);
}

fn clamp_to_surface(position: vec3<f32>) -> vec3<f32> {
    let min_radius = atmosphere.bottom_radius + EPSILON_M;
    let r = length(position);
    if (r < min_radius) {
        let up = safe_normalize(position);
        return up * min_radius;
    }
    return position;
}

fn get_view_position() -> vec3<f32> {
    // Matches `functions.wgsl::get_view_position()` to keep anchor consistent.
    let world_pos = view.world_position * settings.scene_units_to_m + vec3(0.0, atmosphere.bottom_radius, 0.0);
    return clamp_to_surface(world_pos);
}

struct LightBasis {
    x: vec3<f32>,
    y: vec3<f32>,
};

fn build_light_basis(light_dir: vec3<f32>) -> LightBasis {
    // Pick a vector not parallel to `light_dir`, then build an orthonormal basis.
    // IMPORTANT: keep this selection stable; switching too early causes the basis to “flip”
    // when the light gets moderately close to zenith, which looks like the shadow map drifts/rotates.
    let world_up = vec3(0.0, 1.0, 0.0);
    let a = select(world_up, vec3(1.0, 0.0, 0.0), abs(dot(light_dir, world_up)) > 0.999);
    let x = safe_normalize(cross(a, light_dir));
    let y = cross(light_dir, x);
    return LightBasis(x, y);
}

fn snap_anchor_to_texel_grid(anchor: vec3<f32>, basis: LightBasis, extent: f32, size: vec2<u32>) -> vec3<f32> {
    // Snap the anchor in the shadow-map XY plane to stabilize the map under camera motion.
    // This mirrors Unreal's stabilized shadow-map anchor strategy.
    let res = vec2<f32>(size);
    let texel_size = (2.0 * extent) / max(res, vec2(1.0));

    let ax = dot(anchor, basis.x);
    let ay = dot(anchor, basis.y);

    let snapped_ax = round(ax / texel_size.x) * texel_size.x;
    let snapped_ay = round(ay / texel_size.y) * texel_size.y;

    let dx = snapped_ax - ax;
    let dy = snapped_ay - ay;
    return anchor + basis.x * dx + basis.y * dy;
}

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let size = settings.cloud_shadow_map_size;
    if (gid.x >= size.x || gid.y >= size.y) {
        return;
    }

    // If there is no directional light, write "no clouds".
    if (lights.n_directional_lights == 0u) {
        let far_depth_km = (2.0 * settings.cloud_shadow_map_half_depth) * 0.001;
        textureStore(out_cloud_shadow_map, vec2<i32>(gid.xy), vec4(far_depth_km, 0.0, 0.0, 0.0));
        return;
    }

    // IMPORTANT: trace direction must point from the light toward the scene (light -> surface),
    // matching Unreal's convention. Bevy's `direction_to_light` points from the point toward the
    // light (surface -> light), so we negate it here.
    let trace_dir = safe_normalize(-lights.directional_lights[0].direction_to_light);
    let basis = build_light_basis(trace_dir);

    let extent = settings.cloud_shadow_map_extent;
    let half_depth = settings.cloud_shadow_map_half_depth;
    let strength = settings.cloud_shadow_map_strength;

    // Map texel -> light-space XY in [-extent, extent].
    let uv = (vec2<f32>(gid.xy) + vec2(0.5)) / vec2<f32>(size);
    let xy = (uv - vec2(0.5)) * (2.0 * extent);

    var anchor = get_view_position();
    anchor = snap_anchor_to_texel_grid(anchor, basis, extent, size);
    let ray_origin = anchor + basis.x * xy.x + basis.y * xy.y - trace_dir * half_depth;
    let max_t = 2.0 * half_depth;
    let max_t_km = max_t * 0.001;

    // DEBUG: write a predictable gradient to validate dispatch + bindings + sampling.
    // - R stores a *normalized* value in [0,1] (uv.x), so the debug view should show a full-width gradient
    //   even if `cloud_shadow_map_half_depth` is being interpreted differently elsewhere.
    // - G shows `uv.y`
    // - B/A unused
    if (CLOUD_SHADOW_DEBUG_WRITE_UV) {
        textureStore(out_cloud_shadow_map, vec2<i32>(gid.xy), vec4(uv.x, uv.y, 0.0, 0.0));
        return;
    }

    // Intersect the ray with the cloud layer shell and clamp to our light volume depth.
    let seg = cloud_layer_segment(ray_origin, trace_dir);
    if (seg.z < 0.5) {
        textureStore(out_cloud_shadow_map, vec2<i32>(gid.xy), vec4(max_t_km, 0.0, 0.0, 0.0));
        return;
    }

    let t_start = max(0.0, seg.x);
    let t_end = min(seg.y, max_t);
    if (t_end <= t_start) {
        textureStore(out_cloud_shadow_map, vec2<i32>(gid.xy), vec4(max_t_km, 0.0, 0.0, 0.0));
        return;
    }

    let n = max(1u, settings.cloud_shadow_map_samples);
    let dt = (t_end - t_start) / f32(n);

    let sigma_t_per_density = get_cloud_scattering_coeff() + get_cloud_absorption_coeff();

    var front_depth = max_t;
    var sum_ext = 0.0;
    var count_ext = 0.0;
    var max_optical_depth = 0.0;

    // Avoid “phantom” front depths from tiny numerical densities, which can cast shadows
    // in empty space on the sun-facing side of clouds.
    // Units: optical depth is unitless.
    const HIT_OPTICAL_DEPTH_THRESHOLD: f32 = 1e-3;

    for (var i = 0u; i < n; i += 1u) {
        let t = t_start + (f32(i) + 0.5) * dt;
        let p = ray_origin + trace_dir * t;
        let r = length(p);
        let density = get_cloud_medium_density(r, p);
        if (density > 0.0) {
            let extinction = density * sigma_t_per_density; // 1/m
            let od = extinction * dt;
            if (od > HIT_OPTICAL_DEPTH_THRESHOLD) {
                if (front_depth == max_t) {
                    front_depth = t;
                }
                sum_ext += extinction;
                count_ext += 1.0;
                max_optical_depth += od;
            }
        }
    }

    let mean_ext = select(0.0, sum_ext / count_ext, count_ext > 0.0);

    // Apply user strength scaling (matches Unreal's "strength" behavior).
    let out_mean_ext = mean_ext * strength;
    let out_max_od = max_optical_depth * strength;

    // Store front depth in kilometers to avoid fp16 overflow at large volumes.
    textureStore(out_cloud_shadow_map, vec2<i32>(gid.xy), vec4(front_depth * 0.001, out_mean_ext, out_max_od, 0.0));
}



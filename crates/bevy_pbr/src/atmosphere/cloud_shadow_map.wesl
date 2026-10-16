
// Computes an Unreal-style cloud shadow map storing:
// R: front depth (kilometers) from the light-volume near plane (stored in km to fit fp16 range)
// G: mean extinction (1/m)
// B: max optical depth (unitless)
//
// This is later sampled at arbitrary world points to cheaply compute
// transmittance: T = exp(-tau) where tau = mean_ext * (d_sample - d_front),
// clamped to max optical depth.

import package::atmosphere::bindings::{atmosphere, settings, view, lights};
import package::atmosphere::clouds::{
    cloud_layer_segment,
    get_cloud_medium_density_coarse,
    get_cloud_medium_density_lod,
    get_cloud_scattering_coeff,
    get_cloud_absorption_coeff,
};

@group(0) @binding(25) var stbn_texture: texture_2d_array<f32>;
@group(0) @binding(13) var out_cloud_shadow_map: texture_storage_2d<rgba16float, write>;

const EPSILON_M: f32 = 1.0;

// Simple deterministic hash utilities (no frame index required).
// Used to jitter integration per shadow-map texel to mitigate banding.
fn hash_u32(x_in: u32) -> u32 {
    var x = x_in;
    x ^= x >> 16u;
    x *= 0x7feb352du;
    x ^= x >> 15u;
    x *= 0x846ca68bu;
    x ^= x >> 16u;
    return x;
}

fn hash_2d_to_01(p: vec2<u32>, salt: u32) -> f32 {
    let h = hash_u32(p.x ^ (hash_u32(p.y) + 0x9e3779b9u) ^ salt);
    // Convert to [0,1). 2^32 as f32 is representable.
    return f32(h) * (1.0 / 4294967296.0);
}

fn safe_normalize(v: vec3<f32>) -> vec3<f32> {
    let l2 = dot(v, v);
    if (l2 <= 1e-12) {
        return vec3(0.0, 1.0, 0.0);
    }
    return v * inverseSqrt(l2);
}

fn clamp_to_surface(position: vec3<f32>) -> vec3<f32> {
    let min_radius = atmosphere.inner_radius + EPSILON_M;
    let r = length(position);
    if (r < min_radius) {
        let up = safe_normalize(position);
        return up * min_radius;
    }
    return position;
}

fn get_view_position() -> vec3<f32> {
    // Matches `functions.wgsl::get_view_position()` to keep anchor consistent.
    let atmo_pos = (atmosphere.world_to_atmosphere * vec4(view.world_position, 1.0)).xyz;
    return clamp_to_surface(atmo_pos);
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
    // Snap in large steps (UE5 r.VolumetricCloud.ShadowMap.SnapLength, 20 km), a whole number of
    // texels: the map stays fixed in the world while the camera flies, so shadow texels do not
    // crawl (snapping per texel shifted the time-sliced, temporally filtered map every ~125 m).
    // A quarter of the half-extent keeps the camera in the map's central half.
    let texel = (2.0 * extent) / max(res, vec2(1.0));
    let texel_size = max(round(0.25 * extent / texel), vec2(1.0)) * texel;

    let ax = dot(anchor, basis.x);
    let ay = dot(anchor, basis.y);

    let snapped_ax = round(ax / texel_size.x) * texel_size.x;
    let snapped_ay = round(ay / texel_size.y) * texel_size.y;

    let dx = snapped_ax - ax;
    let dy = snapped_ay - ay;
    return anchor + basis.x * dx + basis.y * dy;
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

const CLOUD_SHADOW_NOISE_MIP: f32 = 2.0;

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) thread_id: vec3<u32>) {
    let size = settings.cloud_shadow_map_size;
    // With time-slicing, each thread traces one texel of an N x N tile; the other texels are
    // reprojected from history by the temporal pass.
    let interleave = max(1u, settings.cloud_shadow_map_update_interleave);
    let slot = cloud_shadow_interleave_slot(view.frame_count, interleave);
    let gid = vec3<u32>(thread_id.xy * interleave + slot, thread_id.z);
    if (gid.x >= size.x || gid.y >= size.y) {
        return;
    }

    // If there is no directional light, write "no clouds".
    if (lights.n_directional_lights == 0u) {
        let half_depth = settings.cloud_shadow_map_extent * 2.0;
        let far_depth_km = (2.0 * half_depth) * 0.001;
        textureStore(out_cloud_shadow_map, vec2<i32>(gid.xy), vec4(far_depth_km, 0.0, 0.0, 0.0));
        return;
    }

    // IMPORTANT: trace direction must point from the light toward the scene (light -> surface),
    // matching Unreal's convention. Bevy's `direction_to_light` points from the point toward the
    // light (surface -> light), so we negate it here.
    let trace_dir = safe_normalize(-lights.directional_lights[0].direction_to_light);
    let basis = build_light_basis(trace_dir);

    let extent = settings.cloud_shadow_map_extent;
    // Unreal-style: derive depth range from extent (prevents pathological far near-plane positions at low sun).
    let half_depth = extent * 2.0;
    let strength = settings.cloud_shadow_map_strength;

    // Map texel -> light-space XY in [-extent, extent].
    let uv = (vec2<f32>(gid.xy) + vec2(0.5)) / vec2<f32>(size);
    let xy = (uv - vec2(0.5)) * (2.0 * extent);

    var anchor = get_view_position();
    anchor = snap_anchor_to_texel_grid(anchor, basis, extent, size);
    let ray_origin = anchor + basis.x * xy.x + basis.y * xy.y - trace_dir * half_depth;
    let max_t = 2.0 * half_depth;
    let max_t_km = max_t * 0.001;

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

    // More samples when the sun is near the horizon (Unreal-style):
    // the ray travels a lot longer through the cloud shell, so a fixed sample count can miss
    // density entirely, producing sweeping “no shadow” bands as the sun moves.
    //
    // Unreal uses:
    //   HorizonFactor = clamp(0.2 / abs(dot(PlanetUp, -LightDir)), 0, 1)
    //   SampleCount *= lerp(1, HorizonMultiplier, HorizonFactor)
    //
    // Here we approximate PlanetUp using the anchor up vector (good enough for ground/near-ground views).
    let base_n = max(1u, settings.cloud_shadow_map_samples);
    let anchor_up = safe_normalize(anchor);
    let mu = abs(dot(anchor_up, trace_dir));
    // Only grazing rays need many samples (hero/up views were unchanged down to 16 samples;
    // sun:3 degraded below ~96).
    let horizon_factor = clamp(0.08 / max(mu, 1e-3), 0.0, 1.0);
    const HORIZON_MULT: f32 = 4.0;
    let n = min(512u, max(1u, u32(ceil(f32(base_n) * mix(1.0, HORIZON_MULT, horizon_factor)))));
    // let n = 24u;
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
        // Jittered + stratified: one sample per stratum, with *per-step* jitter.
        // Per-step jitter reduces “streaky” structure compared to using a single phase shift
        // across all steps in a texel.
        var j: f32;
        let stbn_dims = textureDimensions(stbn_texture);
        if (all(stbn_dims > vec2(1u))) {
            let stbn_layers = textureNumLayers(stbn_texture);
            let stbn_layer = i32(view.frame_count % u32(stbn_layers));
            let stbn_noise = textureLoad(stbn_texture, vec2<i32>(gid.xy) % vec2<i32>(stbn_dims), stbn_layer, 0);
            // j = fract(stbn_noise.r + f32(i) * 0.618033988749895);
            j = fract(0.0 + f32(i) * 0.618033988749895);
        } else {
            j = hash_2d_to_01(gid.xy, 0x1000u + i);
        }
        let t = t_start + (f32(i) + j) * dt;
        let p = ray_origin + trace_dir * t;
        let r = length(p);
        // Coarse (weather x profile) only gates empty space; the map integrates the real noise
        // density at a coarse mip. Integrating the coarse proxy itself overestimated occlusion
        // (63 % of the map below 1 % transmittance at 35 % coverage) -> black ground / dark haze.
        var density = 0.0;
        if (get_cloud_medium_density_coarse(r, p) > 0.0) {
            density = get_cloud_medium_density_lod(r, p, CLOUD_SHADOW_NOISE_MIP);
        }
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

    textureStore(out_cloud_shadow_map, vec2<i32>(gid.xy), vec4(front_depth * 0.001, out_mean_ext, out_max_od, 0.0));
}



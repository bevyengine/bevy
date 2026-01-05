// Cloud rendering functions using 3D FBM noise
#define_import_path bevy_pbr::atmosphere::clouds

#import bevy_render::maths::{PI, ray_sphere_intersect}
#import bevy_pbr::utils::interleaved_gradient_noise
#import bevy_pbr::atmosphere::{
    types::{Atmosphere, AtmosphereSettings},
    bindings::{settings, atmosphere},
    functions::{
        get_local_r,
    },
}

struct CloudLayer {
    cloud_layer_start: f32,
    cloud_layer_end: f32,
    cloud_density: f32,
    cloud_absorption: f32,
    cloud_scattering: f32,
    noise_scale: f32,
    noise_offset: vec3<f32>,
    detail_noise_scale: f32,
    detail_strength: f32,
}

@group(0) @binding(14) var<uniform> cloud_layer: CloudLayer;
@group(0) @binding(15) var noise_texture_2d: texture_2d<f32>;
@group(0) @binding(16) var noise_sampler_3d: sampler;

/// Sample the 2D cloud coverage texture (XZ plane) at a given world position.
fn sample_cloud_coverage_noise_at_scale(world_pos: vec3<f32>, noise_scale: f32) -> f32 {
    // Convert world position to noise texture coordinates in XZ.
    // (Repeat wrapping is configured on the sampler.)
    let uv = (world_pos.xz + cloud_layer.noise_offset.xz) / noise_scale;
    return textureSampleLevel(noise_texture_2d, noise_sampler_3d, uv, 0.0).r;
}

/// Get cloud scattering coefficient per unit density
fn get_cloud_scattering_coeff() -> f32 {
    return cloud_layer.cloud_scattering;
}

/// Get cloud absorption coefficient per unit density
fn get_cloud_absorption_coeff() -> f32 {
    return cloud_layer.cloud_absorption;
}

// PERF/DEBUG: Hardcoded repeating sphere SDF volume (no texture fetches).
//
// This is intended for debugging raymarching performance and lighting/shadowing:
// - Deterministic shape
// - Cheap evaluation
// - Soft boundary (stable under undersampling)
//
// All parameters are intentionally hardcoded in shader code.

const CELL_SIZE_M: f32 = 16000.0;     // spacing between sphere centers (meters)
const RADIUS_M: f32 = 2500.0;        // sphere radius (meters)
const SOFTNESS_M: f32 = 500.0;       // boundary softness (meters)

// Debug toggle: when enabled, use a single sphere centered in the middle of the cloud layer
// instead of a repeating cell volume. This is useful to validate coordinate systems and
// shadow-map alignment.
const CLOUD_DEBUG_SINGLE_SPHERE: bool = false;

fn debug_sphere_center() -> vec3<f32> {
    // Place the sphere at scene "origin" in XZ, and centered vertically in the cloud layer.
    // Note: the atmosphere coordinate system is planet-centered; y points "up" locally.
    let center_r = 0.5 * (cloud_layer.cloud_layer_start + cloud_layer.cloud_layer_end);
    return vec3(0.0, center_r, 0.0);
}

/// Cloud shape / coverage term in [0, 1].
/// This is *not* a physical density by itself: it is the normalized field used to
/// shape the cloud volume (noise + height falloff).
fn get_cloud_coverage(r: f32, world_pos: vec3<f32>) -> f32 {
    // Check if we're within the cloud layer
    if (r < cloud_layer.cloud_layer_start || r > cloud_layer.cloud_layer_end) {
        return 0.0;
    }
    
    if (CLOUD_DEBUG_SINGLE_SPHERE) {
        let c = debug_sphere_center();
        let sdf = length(world_pos - c) - RADIUS_M;
        // WGSL smoothstep is undefined if edge0 >= edge1, so use a standard form:
        // inside (sdf << 0) => 1, outside (sdf >> 0) => 0.
        let density = 1.0 - smoothstep(-SOFTNESS_M, SOFTNESS_M, sdf);
        return clamp(density, 0.0, 1.0);
    } else {
        // Calculate height factor within cloud layer (0 at bottom, 1 at top)
        let layer_thickness = cloud_layer.cloud_layer_end - cloud_layer.cloud_layer_start;
        let height_in_layer = r - cloud_layer.cloud_layer_start;
        // Repeat only in XZ (not Y): keep sphere centers at a fixed height inside the layer.
        // This avoids spheres being “sliced” by the top/bottom of the cloud shell due to Y tiling.
        let center_y = 0.5 * layer_thickness;
        let y_rel = height_in_layer - center_y;

        // Repeat in XZ cells, center each cell at the origin.
        let cell_xz = fract(world_pos.xz / CELL_SIZE_M) - vec2(0.5);
        let q = vec3(cell_xz * CELL_SIZE_M, y_rel);

        // Sphere SDF: negative inside, positive outside.
        let sdf = length(q) - RADIUS_M;

        // Convert SDF to density with a smooth boundary.
        // sdf <= -SOFTNESS => ~1, sdf >= +SOFTNESS => ~0
        let sphere_density = 1.0 - smoothstep(-SOFTNESS_M, SOFTNESS_M, sdf);

        return clamp(sphere_density, 0.0, 1.0);
    }
}

/// Returns (density, grad_mag) for the current debug cloud field.
/// - density is the normalized coverage term in [0,1]
/// - grad_mag is an estimate of |∇density| in 1/m, useful for adaptive stepping.
fn sample_cloud_field_density_and_grad(r: f32, world_pos: vec3<f32>) -> vec2<f32> {
    // Outside cloud layer => empty and flat field.
    if (r < cloud_layer.cloud_layer_start || r > cloud_layer.cloud_layer_end) {
        return vec2(0.0, 0.0);
    }

    var sdf: f32;
    var height_fade: f32 = 1.0;

    if (CLOUD_DEBUG_SINGLE_SPHERE) {
        let c = debug_sphere_center();
        sdf = length(world_pos - c) - RADIUS_M;
    } else {
        let layer_thickness = cloud_layer.cloud_layer_end - cloud_layer.cloud_layer_start;
        let height_in_layer = r - cloud_layer.cloud_layer_start;
        // Repeat only in XZ (not Y), match `get_cloud_coverage()`.
        let center_y = 0.5 * layer_thickness;
        let y_rel = height_in_layer - center_y;
        let cell_xz = fract(world_pos.xz / CELL_SIZE_M) - vec2(0.5);
        let q = vec3(cell_xz * CELL_SIZE_M, y_rel);
        sdf = length(q) - RADIUS_M;
        height_fade = 1.0;
    }

    // density = smoothstep(SOFTNESS_M, -SOFTNESS_M, sdf)
    // derivative of smoothstep(edge0, edge1, x) is:
    // 6t(1-t)/(edge1-edge0), where t = clamp((x-edge0)/(edge1-edge0), 0,1)
    // We implement density = 1 - smoothstep(-SOFTNESS, +SOFTNESS, sdf) to avoid edge reversal.
    let edge0 = -SOFTNESS_M;
    let edge1 = SOFTNESS_M;
    let denom = (edge1 - edge0); // 2*SOFTNESS_M
    let t = clamp((sdf - edge0) / denom, 0.0, 1.0);
    let smoothen = t * t * (3.0 - 2.0 * t);
    let sphere_density = 1.0 - smoothen;
    // |d/dsdf (1 - smoothstep)| == |d/dsdf smoothstep|
    let d_sphere_density_dsdf = 6.0 * t * (1.0 - t) / denom;

    // ∇sdf has magnitude 1 almost everywhere (except at the exact center); so |∇density| ≈ |d density/d sdf|
    let density = clamp(sphere_density * height_fade, 0.0, 1.0);
    let grad_mag = d_sphere_density_dsdf * height_fade;

    return vec2(density, grad_mag);
}

/// Returns (start_t, end_t, valid) for the segment of the ray inside the cloud layer shell.
/// - start_t/end_t are distances along the ray direction (meters), relative to ray_origin.
/// - valid is 1.0 if the segment exists, 0.0 otherwise.
fn cloud_layer_segment(ray_origin: vec3<f32>, ray_dir: vec3<f32>) -> vec3<f32> {
    // IMPORTANT: do the ray/sphere intersection math in kilometers to improve numerical stability.
    // In meters, `r` is ~6.3e6 and for grazing angles the discriminant can become ill-conditioned,
    // producing "banded" invalid intersections as the sun elevation changes.
    // Doing the math in km keeps magnitudes ~6.3e3 and greatly reduces cancellation.
    const M_TO_KM: f32 = 0.001;
    const KM_TO_M: f32 = 1000.0;

    let r_km = length(ray_origin) * M_TO_KM;
    let up = normalize(ray_origin);
    let mu = dot(ray_dir, up);

    let bottom_radius_km = cloud_layer.cloud_layer_start * M_TO_KM;
    let top_radius_km = cloud_layer.cloud_layer_end * M_TO_KM;

    // Unreal-style intersection selection:
    // Compute intersections with the top and bottom spheres, then derive a single [TMin, TMax]
    // interval from carefully selected roots. This avoids sign/branch issues at grazing angles.
    let t_top = ray_sphere_intersect(r_km, mu, top_radius_km);
    if (t_top.x < 0.0 && t_top.y < 0.0) {
        return vec3(0.0, 0.0, 0.0);
    }
    let t_bottom = ray_sphere_intersect(r_km, mu, bottom_radius_km);

    var t_min = t_top.x;
    var t_max = t_top.y;

    // If we also intersect the bottom sphere, combine both.
    if (!(t_bottom.x < 0.0 && t_bottom.y < 0.0)) {
        // If we see both intersections in front of us, keep the min/closest, otherwise the max/furthest.
        var temp_top = select(max(t_top.x, t_top.y), min(t_top.x, t_top.y), (t_top.x > 0.0) && (t_top.y > 0.0));
        var temp_bottom = select(max(t_bottom.x, t_bottom.y), min(t_bottom.x, t_bottom.y), (t_bottom.x > 0.0) && (t_bottom.y > 0.0));

        if ((t_bottom.x > 0.0) && (t_bottom.y > 0.0)) {
            // If we can see the bottom of the layer, make sure we use the camera (0) or the closest top intersection.
            temp_top = max(0.0, min(t_top.x, t_top.y));
        }

        t_min = min(temp_bottom, temp_top);
        t_max = max(temp_bottom, temp_top);
    }

    t_min = max(0.0, t_min);
    t_max = max(0.0, t_max);
    let valid = (t_max > t_min);

    // Convert km back to meters for callers.
    return vec3(t_min * KM_TO_M, t_max * KM_TO_M, select(0.0, 1.0, valid));
}

/// Cloud *medium density* used for extinction / scattering integration.
/// This is the normalized coverage term scaled by `cloud_layer.cloud_density`.
fn get_cloud_medium_density(r: f32, world_pos: vec3<f32>) -> f32 {
    return get_cloud_coverage(r, world_pos) * cloud_layer.cloud_density;
}

/// Raymarch through clouds towards the sun to compute volumetric shadow
/// Returns the light transmittance factor [0,1] where 0 = fully shadowed, 1 = no shadow
/// Properly handles viewer inside clouds and grazing angles
fn compute_cloud_shadow(
    world_pos: vec3<f32>,
    sun_dir: vec3<f32>,
    steps: u32,
    pixel_coords: vec2<f32>,
) -> f32 {
    // Early exit if clouds are disabled
    if (cloud_layer.cloud_density <= 0.0) {
        return 1.0;
    }

    // March bounds: only integrate within the cloud layer shell along the sun ray.
    let seg = cloud_layer_segment(world_pos, sun_dir);
    if (seg.z < 0.5) {
        return 1.0;
    }

    var march_start = max(seg.x, 0.0);
    var march_end = seg.y;
    if (march_start >= march_end || march_end <= 0.0) {
        return 1.0;
    }

    // Earth (planet) shadow term:
    // If the sun ray hits the planet surface before it exits the cloud layer,
    // the sun is fully occluded and there is no direct lighting.
    let r0 = length(world_pos);
    let up0 = normalize(world_pos);
    let mu0 = dot(sun_dir, up0);
    let ground_i = ray_sphere_intersect(r0, mu0, atmosphere.bottom_radius);
    // `ground_i.x` is the nearest positive intersection along the ray, if any.
    if (ground_i.x > 0.0 && ground_i.x < march_end) {
        return 0.0;
    }

    let march_distance = march_end - march_start;

    // Adaptive step count: long segments need more samples, otherwise we can miss small dense regions
    // (especially with the repeating-sphere debug density field).
    const TARGET_STEP_M: f32 = 1500.0;
    const MAX_SHADOW_STEPS: f32 = 16.0;
    let desired = clamp(ceil(march_distance / TARGET_STEP_M), f32(steps), MAX_SHADOW_STEPS);
    let shadow_steps = max(1u, u32(desired));

    let step_size = march_distance / f32(shadow_steps);
    var optical_depth = 0.0;

    // Raymarch through clouds towards sun with per-step jitter.
    for (var i = 0u; i < shadow_steps; i++) {
        let j = interleaved_gradient_noise(pixel_coords, 100u + i); // [0,1]
        let t = march_start + (f32(i) + j) * step_size;
        let sample_pos = world_pos + sun_dir * t;
        let sample_r = length(sample_pos);

        let density = get_cloud_medium_density(sample_r, sample_pos);
        if (density > 0.0) {
            let extinction = density * (cloud_layer.cloud_scattering + cloud_layer.cloud_absorption);
            optical_depth += extinction * step_size;
            // Early out when essentially fully shadowed
            if (optical_depth > 8.0) {
                return 0.0;
            }
        }
    }

    return exp(-optical_depth);
}

/// Simplified cloud contribution for a single sample point
/// Returns (luminance_added, transmittance_multiplier)
fn sample_cloud_contribution(
    world_pos: vec3<f32>,
    step_size: f32,
) -> vec2<f32> {
    let r = length(world_pos);
    let density = get_cloud_medium_density(r, world_pos);
    
    if (density < 0.01) {
        return vec2(0.0, 1.0);
    }
    
    // Physically correct coefficients (units: m^-1 per unit density)
    let extinction = density * (cloud_layer.cloud_scattering + cloud_layer.cloud_absorption);
    let scattering = density * cloud_layer.cloud_scattering;
    
    // Beer's law
    let transmittance = exp(-extinction * step_size);
    
    // Simple uniform scattering (could be enhanced with actual sun direction)
    let in_scatter = scattering * (1.0 - transmittance);
    
    return vec2(in_scatter, transmittance);
}

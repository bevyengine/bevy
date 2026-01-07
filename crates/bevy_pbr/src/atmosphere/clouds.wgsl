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
@group(0) @binding(18) var perlin_worley_noise_3d: texture_3d<f32>;

fn safe_normalize(v: vec3<f32>) -> vec3<f32> {
    let l2 = dot(v, v);
    if (l2 <= 1e-12) {
        return vec3(0.0, 1.0, 0.0);
    }
    return v * inverseSqrt(l2);
}

// Sample packed cloud noise (RGBA) at a given scale.
// - R: coverage (macro placement)
// - G: bottom type (vertical profile shaping)
// - B: top type (vertical profile shaping)
// - A: detail (erosion / "up-rez")
fn sample_cloud_noise_rgba_at_scale(world_pos: vec3<f32>, noise_scale: f32) -> vec4<f32> {
    // Convert world position to noise texture coordinates in XZ.
    // Apply a small rotation to break up axis-aligned stretching artifacts.
    let rot = mat2x2<f32>(
        0.8660254, -0.5,
        0.5, 0.8660254
    ); // 30 degrees
    let xz = rot * world_pos.xz;
    let uv = (xz + cloud_layer.noise_offset.xz) / noise_scale;
    return textureSampleLevel(noise_texture_2d, noise_sampler_3d, uv, 0.0);
}

fn sample_perlin_worley_3d(world_pos: vec3<f32>, h: f32) -> vec4<f32> {
    // Map world XZ into texture space, and use normalized layer height as the 3rd dimension.
    // A small rotation helps break up axis-aligned repetition.
    let rot = mat2x2<f32>(
        0.8660254, -0.5,
        0.5, 0.8660254
    );
    let xz = rot * world_pos.xz;
    // Scale is tied to the existing noise scale for now to keep tuning surface area small.
    let uv = (xz + cloud_layer.noise_offset.xz) / 1000.0;
    let w = h + cloud_layer.noise_offset.y * 1e-5;
    let uvw = vec3<f32>(uv, w);
    return textureSampleLevel(perlin_worley_noise_3d, noise_sampler_3d, uvw, 0.0);
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

const CELL_SIZE_M: f32 = 4000.0;     // spacing between sphere centers (meters)
const RADIUS_M: f32 = 750.0;        // sphere radius (meters)
const SOFTNESS_M: f32 = 10.0;       // boundary softness (meters)

// Debug toggle: when enabled, use a single sphere centered in the middle of the cloud layer
// instead of a repeating cell volume. This is useful to validate coordinate systems and
// shadow-map alignment.
const CLOUD_DEBUG_SINGLE_SPHERE: bool = false;

// Primary cloud shape toggle:
// - false: use the debug sphere SDF volume (single/repeating) for lighting/debugging
// - true : use noise-based cumulus shaping (envelope + FBM erosion)
const CLOUD_USE_NOISE_SHAPE: bool = true;

// --- Cumulus tuning knobs (shader constants for now) ---
// Increase density and sharpen cloud borders by tightening the coverage threshold band
// and applying a contrast curve.
const CUMULUS_EDGE_THRESHOLD: f32 = 0.75; // higher => fewer, more isolated clouds
const CUMULUS_EDGE_WIDTH: f32 = 0.1;     // smaller => sharper border
const CUMULUS_EDGE_SHARPNESS: f32 = 16.0;  // >1 => steeper transition to "fully inside cloud"

fn debug_sphere_center() -> vec3<f32> {
    // Place the sphere at scene "origin" in XZ, and centered vertically in the cloud layer.
    // Note: the atmosphere coordinate system is planet-centered; y points "up" locally.
    let center_r = 0.5 * (cloud_layer.cloud_layer_start + cloud_layer.cloud_layer_end);
    return vec3(0.0, center_r, 0.0);
}

// --- Noise-based cumulus shaping (NUBIS-like envelope method) ---
//
// We use:
// - a 2D "coverage" field (noise texture) mapped over XZ
// - an envelope profile over height (bottom/top gradients)
// - FBM-like erosion/detail sampled from the same 2D noise with different scales/warps
//
// This keeps the binding footprint unchanged while producing much more cloud-like shapes
// than the sphere debug volume.

fn remap(x: f32, a: f32, b: f32, c: f32, d: f32) -> f32 {
    let t = clamp((x - a) / max(1e-6, b - a), 0.0, 1.0);
    return mix(c, d, t);
}

// --- Vertical profile method (NUBIS-style) ---
//
// In Nubis/UE the "top type" and "bottom type" are used to sample 2D profile lookup textures:
// - x axis: type
// - y axis: height in layer
//
// We don't have those LUT textures bound yet, so we approximate them with analytic curves that are:
// - distinct for top vs bottom
// - smoothly varying with type
//
// If/when we add real LUT textures, these become simple texture samples.
fn cloud_bottom_profile(h: f32, bottom_type: f32) -> f32 {
    // Bottom profile: controls how quickly density builds from the base.
    // Type 0: thin base / slow build.
    // Type 1: thick base / fast build.
    let knee = mix(0.35, 0.08, bottom_type);
    let x = smoothstep(0.0, knee, h);
    let exp = mix(2.4, 0.75, bottom_type);
    return pow(x, exp);
}

fn cloud_top_profile(h: f32, top_type: f32) -> f32 {
    // Top profile: controls how quickly density fades near the cap.
    // Type 0: hard cap (more anvil-ish)
    // Type 1: soft, billowy fade
    let t = 1.0 - h;
    let knee = mix(0.10, 0.45, top_type);
    let x = smoothstep(0.0, knee, t);
    let exp = mix(0.9, 3.2, top_type);
    return pow(x, exp);
}

fn sample_cumulus_shape(r: f32, world_pos: vec3<f32>) -> f32 {
    // Cloud layer normalized height.
    let layer_thickness = cloud_layer.cloud_layer_end - cloud_layer.cloud_layer_start;
    let height_in_layer = r - cloud_layer.cloud_layer_start;
    let h = clamp(height_in_layer / max(1.0, layer_thickness), 0.0, 1.0);

    // --- NUBIS-like Vertical Profile Method ---
    // We start with 2D NDF-style fields:
    // - coverage: controls where clouds form
    // - bottom_type/top_type: controls the vertical profile shape
    //
    // Then:
    // dimensional_profile = vertical_profile * coverage
    let macro_noise = sample_cloud_noise_rgba_at_scale(world_pos, cloud_layer.noise_scale);
    let cov = macro_noise.r;
    let bottom_type = macro_noise.g;
    let top_type = macro_noise.b;

    // Vertical profile (height-dependent density envelope).
    let bottom_profile = cloud_bottom_profile(h, bottom_type);
    let top_profile = cloud_top_profile(h, top_type);
    let vertical_profile = clamp(bottom_profile * top_profile, 0.0, 1.0);

    // Base dimensional profile (before erosion).
    // IMPORTANT: if coverage is purely 2D and thresholded hard, edges become vertical "walls".
    // We fix that by modulating the coverage threshold with height-varying erosion noise.
    var d = vertical_profile;

    // Keep the existing 2D detail channel for edge modulation / small-scale erosion.
    let micro = sample_cloud_noise_rgba_at_scale(world_pos, cloud_layer.detail_noise_scale);
    let detail_n = micro.a;

    // 3D Perlin–Worley shaping: gives true volumetric breakup and avoids the "vertical walls"
    // you get from purely 2D coverage fields.
    let pw = sample_perlin_worley_3d(world_pos, h);
    let worley = pw.yzw;
    // Combine Worley FBM bands (matches the reference weights).
    let wfbm = worley.x * 0.625 + worley.y * 0.125 + worley.z * 0.25;
    var shape3 = remap(pw.x, wfbm - 1.0, 1.0, 0.0, 1.0);
    shape3 = remap(shape3, 0.85, 1.0, 0.0, 1.0);
    shape3 = clamp(shape3, 0.0, 1.0);

    // Coverage mask with height-varying threshold modulation (kills vertical walls).
    let edge0 = CUMULUS_EDGE_THRESHOLD - CUMULUS_EDGE_WIDTH;
    let edge1 = CUMULUS_EDGE_THRESHOLD + CUMULUS_EDGE_WIDTH;
    // Shift coverage by a small amount that varies with height/warped noise.
    // Higher values => more ragged/sculpted cloud boundaries.
    let edge_mod = (detail_n - 0.5) * 1.0 * smoothstep(0.0, 1.0, h);
    // fake cov value for testing (xy sine wave)
    let cov_fake = sin(world_pos.x * 0.0003) * sin(world_pos.z * 0.0003);
    let edge_raw = smoothstep(edge0, edge1, cov + edge_mod + shape3 * .2);
    let edge = pow(edge_raw, CUMULUS_EDGE_SHARPNESS);
    d *= edge;

    return d;
}

/// Cloud shape / coverage term in [0, 1].
/// This is *not* a physical density by itself: it is the normalized field used to
/// shape the cloud volume (noise + height falloff).
fn get_cloud_coverage(r: f32, world_pos: vec3<f32>) -> f32 {
    // Check if we're within the cloud layer
    if (r < cloud_layer.cloud_layer_start || r > cloud_layer.cloud_layer_end) {
        return 0.0;
    }

    if (CLOUD_USE_NOISE_SHAPE) {
        return sample_cumulus_shape(r, world_pos);
    }

    // Debug sphere volume path
    if (CLOUD_DEBUG_SINGLE_SPHERE) {
        let c = debug_sphere_center();
        let sdf = length(world_pos - c) - RADIUS_M;
        let density = 1.0 - smoothstep(-SOFTNESS_M, SOFTNESS_M, sdf);
        return clamp(density, 0.0, 1.0);
    }

    // Repeating spheres in XZ only.
    let layer_thickness = cloud_layer.cloud_layer_end - cloud_layer.cloud_layer_start;
    let height_in_layer = r - cloud_layer.cloud_layer_start;
    let center_y = 0.5 * layer_thickness;
    let y_rel = height_in_layer - center_y;
    let cell_xz = fract(world_pos.xz / CELL_SIZE_M) - vec2(0.5);
    let q = vec3(cell_xz * CELL_SIZE_M, y_rel);
    let sdf = length(q) - RADIUS_M;
    let sphere_density = 1.0 - smoothstep(-SOFTNESS_M, SOFTNESS_M, sdf);
    return clamp(sphere_density, 0.0, 1.0);
}

/// Returns (density, grad_mag) for the current debug cloud field.
/// - density is the normalized coverage term in [0,1]
/// - grad_mag is an estimate of |∇density| in 1/m, useful for adaptive stepping.
fn sample_cloud_field_density_and_grad(r: f32, world_pos: vec3<f32>) -> vec2<f32> {
    // Outside cloud layer => empty and flat field.
    if (r < cloud_layer.cloud_layer_start || r > cloud_layer.cloud_layer_end) {
        return vec2(0.0, 0.0);
    }

    // If we are using the noise shape, approximate gradient magnitude with finite differences
    // in XZ (cheap-ish and good enough for adaptive substepping decisions).
    if (CLOUD_USE_NOISE_SHAPE) {
        let d0 = sample_cumulus_shape(r, world_pos);
        const EPS_M: f32 = 500.0;
        // Sample along local tangent directions to avoid directional bias / skew.
        let up = safe_normalize(world_pos);
        let east = safe_normalize(vec3(-up.z, 0.0, up.x));
        let north = safe_normalize(cross(up, east));
        let dx = sample_cumulus_shape(r, world_pos + east * EPS_M);
        let dz = sample_cumulus_shape(r, world_pos + north * EPS_M);
        // |∇density| ≈ sqrt((dd/dx)^2 + (dd/dz)^2)
        let ddx = abs(dx - d0) / EPS_M;
        let ddz = abs(dz - d0) / EPS_M;
        let grad_mag = sqrt(ddx * ddx + ddz * ddz);
        return vec2(d0, grad_mag);
    }

    // Debug sphere field: analytic gradient proxy from SDF smoothstep.
    var sdf: f32;
    if (CLOUD_DEBUG_SINGLE_SPHERE) {
        let c = debug_sphere_center();
        sdf = length(world_pos - c) - RADIUS_M;
    } else {
        let layer_thickness = cloud_layer.cloud_layer_end - cloud_layer.cloud_layer_start;
        let height_in_layer = r - cloud_layer.cloud_layer_start;
        let center_y = 0.5 * layer_thickness;
        let y_rel = height_in_layer - center_y;
        let cell_xz = fract(world_pos.xz / CELL_SIZE_M) - vec2(0.5);
        let q = vec3(cell_xz * CELL_SIZE_M, y_rel);
        sdf = length(q) - RADIUS_M;
    }

    let edge0 = -SOFTNESS_M;
    let edge1 = SOFTNESS_M;
    let denom = (edge1 - edge0);
    let tt = clamp((sdf - edge0) / denom, 0.0, 1.0);
    let smoothen = tt * tt * (3.0 - 2.0 * tt);
    let sphere_density = 1.0 - smoothen;
    let d_sphere_density_dsdf = 6.0 * tt * (1.0 - tt) / denom;
    let density = clamp(sphere_density, 0.0, 1.0);
    let grad_mag = d_sphere_density_dsdf;
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

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

/// Cloud shape / coverage term in [0, 1].
/// This is *not* a physical density by itself: it is the normalized field used to
/// shape the cloud volume (noise + height falloff).
fn get_cloud_coverage(r: f32, world_pos: vec3<f32>) -> f32 {
    // Check if we're within the cloud layer
    if (r < cloud_layer.cloud_layer_start || r > cloud_layer.cloud_layer_end) {
        return 0.0;
    }
    
    // Calculate height factor within cloud layer (0 at bottom, 1 at top)
    let layer_thickness = cloud_layer.cloud_layer_end - cloud_layer.cloud_layer_start;
    let height_in_layer = r - cloud_layer.cloud_layer_start;
    let height_factor = height_in_layer / layer_thickness;
    
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
    const SOFTNESS_M: f32 = 100.0;       // boundary softness (meters)

    // Use a "cloud-local" coordinate system:
    // XZ are world XZ, Y is height above the cloud base.
    let p = vec3(world_pos.x, height_in_layer, world_pos.z);

    // Repeat in 3D cells, center each cell at the origin.
    let cell = fract(p / CELL_SIZE_M) - vec3(0.5);
    let q = cell * CELL_SIZE_M;

    // Sphere SDF: negative inside, positive outside.
    let sdf = length(q) - RADIUS_M;

    // Convert SDF to density with a smooth boundary.
    // sdf <= -SOFTNESS => ~1, sdf >= +SOFTNESS => ~0
    let sphere_density = smoothstep(SOFTNESS_M, -SOFTNESS_M, sdf);

    // Fade out near top/bottom of layer to reduce hard cuts.
    let height_gradient = 1.0 - abs(height_factor * 2.0 - 1.0);
    let height_fade = smoothstep(0.0, 0.2, height_gradient);

    let density = sphere_density * height_fade;
    
    return clamp(density, 0.0, 1.0);
}

/// Returns (density, grad_mag) for the current debug cloud field.
/// - density is the normalized coverage term in [0,1]
/// - grad_mag is an estimate of |∇density| in 1/m, useful for adaptive stepping.
fn sample_cloud_field_density_and_grad(r: f32, world_pos: vec3<f32>) -> vec2<f32> {
    // Outside cloud layer => empty and flat field.
    if (r < cloud_layer.cloud_layer_start || r > cloud_layer.cloud_layer_end) {
        return vec2(0.0, 0.0);
    }

    let layer_thickness = cloud_layer.cloud_layer_end - cloud_layer.cloud_layer_start;
    let height_in_layer = r - cloud_layer.cloud_layer_start;
    let height_factor = height_in_layer / layer_thickness;

    // Must match the hardcoded debug SDF in get_cloud_coverage.
    const CELL_SIZE_M: f32 = 16000.0;
    const RADIUS_M: f32 = 2500.0;
    const SOFTNESS_M: f32 = 100.0;

    let p = vec3(world_pos.x, height_in_layer, world_pos.z);
    let cell = fract(p / CELL_SIZE_M) - vec3(0.5);
    let q = cell * CELL_SIZE_M;
    let sdf = length(q) - RADIUS_M;

    // density = smoothstep(SOFTNESS_M, -SOFTNESS_M, sdf)
    // derivative of smoothstep(edge0, edge1, x) is:
    // 6t(1-t)/(edge1-edge0), where t = clamp((x-edge0)/(edge1-edge0), 0,1)
    let edge0 = SOFTNESS_M;
    let edge1 = -SOFTNESS_M;
    let denom = (edge1 - edge0); // -2*SOFTNESS_M
    let t = clamp((sdf - edge0) / denom, 0.0, 1.0);
    let sphere_density = t * t * (3.0 - 2.0 * t);
    let d_sphere_density_dsdf = abs(6.0 * t * (1.0 - t) / denom);

    let height_gradient = 1.0 - abs(height_factor * 2.0 - 1.0);
    let height_fade = smoothstep(0.0, 0.2, height_gradient);

    // ∇sdf has magnitude 1 almost everywhere (except at the exact center); so |∇density| ≈ |d density/d sdf|
    // We ignore the (small) contribution from height_fade's gradient for this debug estimate.
    let density = clamp(sphere_density * height_fade, 0.0, 1.0);
    let grad_mag = d_sphere_density_dsdf * height_fade;

    return vec2(density, grad_mag);
}

/// Returns (start_t, end_t, valid) for the segment of the ray inside the cloud layer shell.
/// - start_t/end_t are distances along the ray direction (meters), relative to ray_origin.
/// - valid is 1.0 if the segment exists, 0.0 otherwise.
fn cloud_layer_segment(ray_origin: vec3<f32>, ray_dir: vec3<f32>) -> vec3<f32> {
    let r = length(ray_origin);
    let up = normalize(ray_origin);
    let mu = dot(ray_dir, up);

    // ray_sphere_intersect returns vec2(near_t, far_t)
    let bottom_i = ray_sphere_intersect(r, mu, cloud_layer.cloud_layer_start);
    let top_i = ray_sphere_intersect(r, mu, cloud_layer.cloud_layer_end);

    var start_t = 0.0;
    var end_t = 0.0;
    var valid = false;

    if (r < cloud_layer.cloud_layer_start) {
        // Below layer: enter at bottom (far), exit at top (far)
        if (bottom_i.y > 0.0 && top_i.y > bottom_i.y) {
            start_t = bottom_i.y;
            end_t = top_i.y;
            valid = true;
        }
    } else if (r < cloud_layer.cloud_layer_end) {
        // Inside layer: exit at top (far) if going outward, else exit at bottom (near)
        if (mu >= 0.0) {
            if (top_i.y > 0.0) {
                start_t = 0.0;
                end_t = top_i.y;
                valid = true;
            }
        } else {
            if (bottom_i.x > 0.0) {
                start_t = 0.0;
                end_t = bottom_i.x;
                valid = true;
            }
        }
    } else {
        // Above layer: enter at top (near), exit at bottom (near)
        if (top_i.x > 0.0 && bottom_i.x > top_i.x) {
            start_t = top_i.x;
            end_t = bottom_i.x;
            valid = true;
        }
    }

    return vec3(start_t, end_t, select(0.0, 1.0, valid));
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

    let march_distance = march_end - march_start;

    // Adaptive step count: long segments need more samples, otherwise we can miss small dense regions
    // (especially with the repeating-sphere debug density field).
    const TARGET_STEP_M: f32 = 1500.0;
    const MAX_SHADOW_STEPS: f32 = 64.0;
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

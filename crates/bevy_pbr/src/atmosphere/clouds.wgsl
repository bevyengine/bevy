// Cloud rendering functions using 3D FBM noise
#define_import_path bevy_pbr::atmosphere::clouds

#import bevy_render::maths::ray_sphere_intersect
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
}

@group(0) @binding(14) var<uniform> cloud_layer: CloudLayer;
@group(0) @binding(15) var noise_texture_3d: texture_3d<f32>;
@group(0) @binding(16) var noise_sampler_3d: sampler;

/// Sample the 3D noise texture at a world position
fn sample_cloud_noise(world_pos: vec3<f32>) -> f32 {
    // Convert world position to noise texture coordinates
    let noise_coords = (world_pos + cloud_layer.noise_offset) / cloud_layer.noise_scale;
    
    // Sample the 3D noise texture with wrapping
    return textureSampleLevel(noise_texture_3d, noise_sampler_3d, noise_coords, 0.0).r;
}

/// Get cloud scattering coefficient per unit density
fn get_cloud_scattering_coeff() -> f32 {
    return cloud_layer.cloud_scattering;
}

/// Get cloud absorption coefficient per unit density
fn get_cloud_absorption_coeff() -> f32 {
    return cloud_layer.cloud_absorption;
}

/// Get cloud density at a given position (in local atmosphere space)
fn get_cloud_density(r: f32, world_pos: vec3<f32>) -> f32 {
    // Check if we're within the cloud layer
    if (r < cloud_layer.cloud_layer_start || r > cloud_layer.cloud_layer_end) {
        return 0.0;
    }
    
    // Calculate height factor within cloud layer (0 at bottom, 1 at top)
    let layer_thickness = cloud_layer.cloud_layer_end - cloud_layer.cloud_layer_start;
    let height_in_layer = r - cloud_layer.cloud_layer_start;
    let height_factor = height_in_layer / layer_thickness;
    
    // Sample noise
    var noise_value = sample_cloud_noise(world_pos);
    noise_value = clamp(pow(noise_value, 3.0), 0.0, 1.0);
    
    // Apply contrast remapping to create sharper cloud boundaries
    // This creates more contrast between cloud/no-cloud areas by remapping mid-values
    let contrast_threshold = 0.3; // Controls how much contrast (lower = sharper edges, range: 0.0-0.5)
    noise_value = smoothstep(contrast_threshold, 1.0 - contrast_threshold, noise_value);
    
    // Height-based density falloff (clouds denser in middle of layer)
    let height_gradient = 1.0 - abs(height_factor * 2.0 - 1.0);
    let height_multiplier = smoothstep(0.0, 0.3, height_gradient) * smoothstep(1.0, 0.6, height_gradient);
    
    // Combine noise with height gradient
    // Density is normalized to [0, 1] for physically correct scattering coefficients
    // where coefficients represent values per unit density
    let density = noise_value * height_multiplier;
    
    return clamp(density, 0.0, 1.0);
}

struct CloudSample {
    density: f32,
    scattering: f32,
    absorption: f32,
}

/// Ray march through the cloud layer
fn raymarch_clouds(
    ray_origin: vec3<f32>,
    ray_dir: vec3<f32>,
    max_distance: f32,
    steps: u32,
    pixel_coords: vec2<f32>,
) -> vec4<f32> {
    // Early exit if clouds are disabled (density is 0)
    if (cloud_layer.cloud_density <= 0.0) {
        return vec4(0.0);
    }
    
    let r = length(ray_origin);
    let mu = dot(ray_dir, normalize(ray_origin));
    
    // Find intersection with cloud layer spheres
    // ray_sphere_intersect returns vec2(near_t, far_t)
    let cloud_bottom_intersect = ray_sphere_intersect(r, mu, cloud_layer.cloud_layer_start);
    let cloud_top_intersect = ray_sphere_intersect(r, mu, cloud_layer.cloud_layer_end);
    
    // Determine ray march bounds through the cloud layer
    var march_start = 0.0;
    var march_end = max_distance;
    
    if (r < cloud_layer.cloud_layer_start) {
        // Below cloud layer - march from cloud bottom to cloud top
        if (cloud_bottom_intersect.y < 0.0) {
            return vec4(0.0); // Ray doesn't hit cloud layer
        }
        march_start = max(0.0, cloud_bottom_intersect.y);
        march_end = min(max_distance, cloud_top_intersect.y);
    } else if (r < cloud_layer.cloud_layer_end) {
        // Inside cloud layer
        march_start = 0.0;
        march_end = min(max_distance, select(cloud_top_intersect.y, cloud_bottom_intersect.x, mu < 0.0));
    } else {
        // Above cloud layer - march from cloud top to cloud bottom
        if (cloud_top_intersect.x < 0.0) {
            return vec4(0.0); // Ray doesn't hit cloud layer
        }
        march_start = max(0.0, cloud_top_intersect.x);
        march_end = min(max_distance, cloud_bottom_intersect.x);
    }
    
    if (march_start >= march_end) {
        return vec4(0.0);
    }
    
    let march_distance = march_end - march_start;
    let step_size = march_distance / f32(steps);
    
    var cloud_color = vec3(0.0);
    var transmittance = 1.0;
    
    // Generate noise offset for temporal jittering (reduces banding)
    let jitter = interleaved_gradient_noise(pixel_coords, 0u);
    
    // Ray march through cloud layer
    for (var i = 0u; i < steps; i++) {
        if (transmittance < 0.01) {
            break;
        }
        
        // Add jitter to sample position to reduce banding artifacts
        let t = march_start + (f32(i) + jitter) * step_size;
        let sample_pos = ray_origin + ray_dir * t;
        let r = length(sample_pos);
        
        let density = get_cloud_density(r, sample_pos);
        
        if (density > 0.01) {
            // Physically correct coefficients (units: m^-1 per unit density)
            // Density is normalized [0, 1], coefficients represent actual physical values
            let extinction = density * (cloud_layer.cloud_scattering + cloud_layer.cloud_absorption);
            let scattering = density * cloud_layer.cloud_scattering;
            
            // Beer's law for transmittance
            let sample_transmittance = exp(-extinction * step_size);
            
            // Simple lighting (could be improved with light ray marching)
            let light_energy = 1.0; // Simplified - should sample actual lighting
            
            // In-scattering contribution
            // Use safe division to avoid divide-by-zero
            if (extinction > 0.0001) {
                cloud_color += light_energy * scattering * transmittance * (1.0 - sample_transmittance) / extinction;
            }
            
            // Update transmittance
            transmittance *= sample_transmittance;
        }
    }
    
    return vec4(cloud_color, 1.0 - transmittance);
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
    
    let r = length(world_pos);
    let up = normalize(world_pos);
    let mu = dot(sun_dir, up);
    
    // Find intersection with cloud layer spheres in sun direction
    let cloud_bottom_intersect = ray_sphere_intersect(r, mu, cloud_layer.cloud_layer_start);
    let cloud_top_intersect = ray_sphere_intersect(r, mu, cloud_layer.cloud_layer_end);
    
    // Determine actual march bounds through cloud layer toward sun
    var march_start = 0.0;
    var march_end = 0.0;
    var valid_intersection = false;
    
    if (r < cloud_layer.cloud_layer_start) {
        // Below clouds - march from cloud bottom to top
        if (cloud_bottom_intersect.y > 0.0 && cloud_top_intersect.y > cloud_bottom_intersect.y) {
            march_start = cloud_bottom_intersect.y;
            march_end = cloud_top_intersect.y;
            valid_intersection = true;
        }
    } else if (r < cloud_layer.cloud_layer_end) {
        // Inside cloud layer - march to exit boundary in sun direction
        if (mu >= 0.0) {
            // Ray going upward/outward - exit at top
            if (cloud_top_intersect.y > 0.0) {
                march_start = 0.0;
                march_end = cloud_top_intersect.y;
                valid_intersection = true;
            }
        } else {
            // Ray going downward/inward - exit at bottom
            if (cloud_bottom_intersect.x > 0.0) {
                march_start = 0.0;
                march_end = cloud_bottom_intersect.x;
                valid_intersection = true;
            }
        }
    } else {
        // Above clouds - march from cloud top to bottom (backward along ray)
        if (cloud_top_intersect.x > 0.0 && cloud_bottom_intersect.x > cloud_top_intersect.x) {
            march_start = cloud_top_intersect.x;
            march_end = cloud_bottom_intersect.x;
            valid_intersection = true;
        }
    }
    
    if (!valid_intersection || march_start >= march_end || march_end <= 0.0) {
        return 1.0;
    }
    
    let march_distance = march_end - march_start;
    let step_size = march_distance / f32(steps);
    let jitter = interleaved_gradient_noise(pixel_coords, 1u);
    
    var optical_depth = 0.0;
    
    // Raymarch through clouds towards sun
    for (var i = 0u; i < steps; i++) {
        let t = march_start + (f32(i) + jitter) * step_size;
        let sample_pos = world_pos + sun_dir * t;
        let sample_r = length(sample_pos);
        
        // Sample density (get_cloud_density already handles bounds)
        let density = get_cloud_density(sample_r, sample_pos);
        
        if (density > 0.01) {
            // Physically correct coefficients (units: m^-1 per unit density)
            let extinction = density * (cloud_layer.cloud_scattering + cloud_layer.cloud_absorption);
            optical_depth += extinction * step_size;
        }
    }
    
    // Beer-Lambert law for shadow transmission
    return exp(-optical_depth);
}

/// Simplified cloud contribution for a single sample point
/// Returns (luminance_added, transmittance_multiplier)
fn sample_cloud_contribution(
    world_pos: vec3<f32>,
    step_size: f32,
) -> vec2<f32> {
    let r = length(world_pos);
    let density = get_cloud_density(r, world_pos);
    
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

#import bevy_render::maths::{PI}
#import bevy_pbr::atmosphere::{
    types::{Atmosphere, AtmosphereSettings},
    bindings::{atmosphere, settings},
    functions::{sample_sky_view_lut, direction_world_to_atmosphere}
}

@group(0) @binding(13) var output: texture_storage_2d_array<rgba16float, write>;

// Convert from cubemap face and UV to direction vector
fn face_uv_to_direction(face: u32, uv: vec2<f32>) -> vec3<f32> {
    // Convert UV from [0,1] to [-1,1]
    let coords = 2.0 * uv - 1.0;
    
    // Generate direction based on face
    var dir: vec3<f32>;
    switch face {
        case 0u: { // +X
            dir = vec3<f32>(1.0, -coords.y, coords.x);
        }
        case 1u: { // -X
            dir = vec3<f32>(-1.0, -coords.y, -coords.x);
        }
        case 2u: { // +Y
            dir = vec3<f32>(coords.x, 1.0, coords.y);
        }
        case 3u: { // -Y
            dir = vec3<f32>(coords.x, -1.0, -coords.y);
        }
        case 4u: { // +Z
            dir = vec3<f32>(coords.x, -coords.y, -1.0);
        }
        default: { // -Z (5)
            dir = vec3<f32>(-coords.x, -coords.y, 1.0);
        }
    }
    
    return normalize(dir);
}

fn radical_inverse_vdc(bits: u32) -> f32 {
    var bits_local = bits;
    bits_local = (bits_local << 16u) | (bits_local >> 16u);
    bits_local = ((bits_local & 0x55555555u) << 1u) | ((bits_local & 0xAAAAAAAAu) >> 1u);
    bits_local = ((bits_local & 0x33333333u) << 2u) | ((bits_local & 0xCCCCCCCCu) >> 2u);
    bits_local = ((bits_local & 0x0F0F0F0Fu) << 4u) | ((bits_local & 0xF0F0F0F0u) >> 4u);
    bits_local = ((bits_local & 0x00FF00FFu) << 8u) | ((bits_local & 0xFF00FF00u) >> 8u);
    return f32(bits_local) * 2.3283064365386963e-10;
}

fn hammersley_2d(i: u32, n: u32) -> vec2<f32> {
    return vec2<f32>(f32(i) / f32(n), radical_inverse_vdc(i));
}

// GGX/Towbridge-Reitz normal distribution function (NDF)
fn distribution_ggx(n_dot_h: f32, roughness: f32) -> f32 {
    let alpha = roughness * roughness;
    let alpha2 = alpha * alpha;
    let denom = n_dot_h * n_dot_h * (alpha2 - 1.0) + 1.0;
    return alpha2 / (PI * denom * denom);
}

fn sample_hemisphere_cosine(xi: vec2<f32>) -> vec3<f32> {
    let phi = 2.0 * PI * xi.x;
    let cos_theta = sqrt(1.0 - xi.y);
    let sin_theta = sqrt(xi.y);
    
    return vec3<f32>(
        cos(phi) * sin_theta,
        cos_theta,
        sin(phi) * sin_theta
    );
}

fn sample_ggx(xi: vec2<f32>, roughness: f32, normal: vec3<f32>) -> vec3<f32> {
    let alpha = roughness * roughness;
    
    let phi = 2.0 * PI * xi.x;
    let cos_theta = sqrt((1.0 - xi.y) / (1.0 + (alpha * alpha - 1.0) * xi.y));
    let sin_theta = sqrt(1.0 - cos_theta * cos_theta);
    
    // Spherical to cartesian coordinates
    let h = vec3<f32>(
        cos(phi) * sin_theta,
        sin(phi) * sin_theta,
        cos_theta
    );
    
    // Tangent-space to world-space
    let up = select(vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(1.0, 0.0, 0.0), abs(normal.y) > 0.999);
    let tangent = normalize(cross(up, normal));
    let bitangent = cross(normal, tangent);
    
    return normalize(tangent * h.x + bitangent * h.y + normal * h.z);
}

// Function to create a local basis where the up direction is aligned with normal
fn create_basis(normal: vec3<f32>) -> mat3x3<f32> {
    let up = select(vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(1.0, 0.0, 0.0), abs(normal.y) > 0.999);
    let tangent = normalize(cross(up, normal));
    let bitangent = cross(normal, tangent);
    return mat3x3<f32>(tangent, bitangent, normal);
}

fn prefilter_environment(dir: vec3<f32>, roughness: f32) -> vec3<f32> {
    let r = atmosphere.bottom_radius;
    let ray_dir_as = direction_world_to_atmosphere(dir);
    
    if (roughness < 0.01) {
        // Just use direct sampling for mirror reflections
        return sample_sky_view_lut(r, ray_dir_as);
    }
    
    let normal = dir;
    let view = dir; // For pre-filtering, view direction is the same as normal for cubemap sampling
    let basis = create_basis(normal);
    
    var prefiltered_color = vec3<f32>(0.0);
    var total_weight = 0.0;
    let num_samples = 1024u;
    
    for (var i = 0u; i < num_samples; i = i + 1u) {
        let xi = hammersley_2d(i, num_samples);
        let h = sample_ggx(xi, roughness, normal);
        let l = normalize(2.0 * dot(view, h) * h - view); // Reflect view around half vector
        
        let n_dot_l = max(dot(normal, l), 0.0);
        
        if (n_dot_l > 0.0) {
            // Sample from sky view LUT in atmospheric space
            let l_as = direction_world_to_atmosphere(l);
            let sample_color = sample_sky_view_lut(r, l_as);
            
            // Apply sample weight based on NDF and incident angle
            prefiltered_color += sample_color * n_dot_l;
            total_weight += n_dot_l;
        }
    }
    
    return prefiltered_color / max(total_weight, 0.001);
}

fn compute_diffuse_irradiance(dir: vec3<f32>) -> vec3<f32> {
    let normal = dir;
    let basis = create_basis(normal);
    
    var irradiance = vec3<f32>(0.0);
    let num_samples = 1024u;
    
    for (var i = 0u; i < num_samples; i = i + 1u) {
        let xi = hammersley_2d(i, num_samples);
        let sample_dir = normalize(basis * sample_hemisphere_cosine(xi));
        
        // Sample from sky view LUT
        let sample_dir_as = direction_world_to_atmosphere(sample_dir);
        let r = atmosphere.bottom_radius;
        let sample_color = sample_sky_view_lut(r, sample_dir_as);
        
        // Cosine weighting is already in the hemisphere sampling
        irradiance += sample_color;
    }
    
    return irradiance * (PI / f32(num_samples));
}

// Generate a test pattern for debugging
fn test_pattern(face: u32, uv: vec2<f32>) -> vec4<f32> {
    // Base colors for each face
    var base_color: vec3<f32>;
    switch face {
        case 0u: { // +X (Right face) - Red
            base_color = vec3<f32>(1.0, 0.2, 0.2);
        }
        case 1u: { // -X (Left face) - Green
            base_color = vec3<f32>(0.2, 1.0, 0.2);
        }
        case 2u: { // +Y (Top face) - Blue
            base_color = vec3<f32>(0.2, 0.2, 1.0);
        }
        case 3u: { // -Y (Bottom face) - Yellow
            base_color = vec3<f32>(1.0, 1.0, 0.2);
        }
        case 4u: { // +Z (Front face) - Cyan
            base_color = vec3<f32>(0.2, 1.0, 1.0);
        }
        default: { // -Z (Back face) - Magenta
            base_color = vec3<f32>(1.0, 0.2, 1.0);
        }
    }
    
    // Checkerboard pattern
    let checker_size = 8.0;
    let cx = floor(uv.x * checker_size);
    let cy = floor(uv.y * checker_size);
    let checker = (cx + cy) % 2.0;
    
    // Direction markers (arrows pointing to center)
    let dist_x = abs(uv.x - 0.5) * 2.0;
    let dist_y = abs(uv.y - 0.5) * 2.0;
    let edge_highlight = step(0.8, max(dist_x, dist_y));
    
    // Face index as text in the center
    let center_marker = step(distance(uv, vec2<f32>(0.5, 0.5)), 0.1);
    
    // Combine patterns
    let pattern = mix(0.5, 1.0, checker);
    var color = base_color * pattern;
    
    // Add edge highlight and center marker
    color = mix(color, vec3<f32>(1.0), edge_highlight * 0.5);
    color = mix(color, vec3<f32>(1.0, 1.0, 1.0), center_marker);
    
    return vec4<f32>(color, 1.0);
}

@compute @workgroup_size(8, 8, 1)
fn specular(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dimensions = textureDimensions(output);
    let slice_index = global_id.z;
    
    if (global_id.x >= dimensions.x || global_id.y >= dimensions.y || slice_index >= 6u) {
        return;
    }
    
    // Calculate normalized UV coordinates for this pixel
    let uv = vec2<f32>(
        (f32(global_id.x) + 0.5) / f32(dimensions.x),
        (f32(global_id.y) + 0.5) / f32(dimensions.y)
    );
    
    let ray_dir_ws = face_uv_to_direction(slice_index, uv);
    let r = atmosphere.bottom_radius;
    let ray_dir_as = direction_world_to_atmosphere(ray_dir_ws);
    let inscattering = sample_sky_view_lut(r, ray_dir_as);
    let color = vec4<f32>(inscattering, 1.0);

    // Write to the correct slice of the output texture
    textureStore(output, vec2<i32>(global_id.xy), i32(slice_index), color);
}

// Entry point for generating diffuse irradiance map
@compute @workgroup_size(8, 8, 1)
fn diffuse(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dimensions = textureDimensions(output);
    let slice_index = global_id.z;
    
    if (global_id.x >= dimensions.x || global_id.y >= dimensions.y || slice_index >= 6u) {
        return;
    }
    
    // Calculate normalized UV coordinates for this pixel
    let uv = vec2<f32>(
        (f32(global_id.x) + 0.5) / f32(dimensions.x),
        (f32(global_id.y) + 0.5) / f32(dimensions.y)
    );
    
    let ray_dir_ws = face_uv_to_direction(slice_index, uv);
    let irradiance = compute_diffuse_irradiance(ray_dir_ws);
    let color = vec4<f32>(irradiance, 1.0);

    // Write to the correct slice of the output texture
    textureStore(output, vec2<i32>(global_id.xy), i32(slice_index), color);
} 


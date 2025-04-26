#import bevy_render::maths::{PI, PI_2};
#import bevy_pbr::lighting::perceptualRoughnessToRoughness;
// Tonemap before filtering to reduce fireflies?
// Pre-expose?

// Uniform buffer for the radiance map
struct PrefilterConstants {
    mip_level: f32,
    sample_count: u32,
    roughness: f32,
    _padding: u32,
}

// Bind groups match the CPU side in prefilter.rs
@group(0) @binding(0) var input_texture: texture_2d_array<f32>;
@group(0) @binding(1) var input_sampler: sampler;
@group(0) @binding(2) var output_texture: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(3) var<uniform> constants: PrefilterConstants;

// Convert UV and face index to direction vector
fn sample_cube_dir(uv: vec2f, face: u32) -> vec3f {
    // Convert from [0,1] to [-1,1]
    let uvc = 2.0 * uv - 1.0;
    
    // Generate direction based on the cube face
    var dir: vec3f;
    switch(face) {
        case 0u: { dir = vec3f( 1.0,  -uvc.y, -uvc.x); } // +X
        case 1u: { dir = vec3f(-1.0,  -uvc.y,  uvc.x); } // -X
        case 2u: { dir = vec3f( uvc.x,  1.0,   uvc.y); } // +Y
        case 3u: { dir = vec3f( uvc.x, -1.0,  -uvc.y); } // -Y
        case 4u: { dir = vec3f( uvc.x, -uvc.y,  1.0);  } // +Z
        case 5u: { dir = vec3f(-uvc.x, -uvc.y, -1.0);  } // -Z
        default: { dir = vec3f(0.0); }
    }
    return normalize(dir);
}

// Convert direction vector to cube face UV
fn dir_to_cube_uv(dir: vec3f) -> vec3u {
    let abs_dir = abs(dir);
    var face: u32 = 0u;
    var uv: vec2f = vec2f(0.0);
    
    // Find the dominant axis to determine face
    if (abs_dir.x >= abs_dir.y && abs_dir.x >= abs_dir.z) {
        // X axis is dominant
        if (dir.x > 0.0) {
            face = 0u; // +X
            uv = vec2f(-dir.z, -dir.y) / dir.x;
        } else {
            face = 1u; // -X
            uv = vec2f(dir.z, -dir.y) / abs_dir.x;
        }
    } else if (abs_dir.y >= abs_dir.x && abs_dir.y >= abs_dir.z) {
        // Y axis is dominant
        if (dir.y > 0.0) {
            face = 2u; // +Y
            uv = vec2f(dir.x, dir.z) / dir.y;
        } else {
            face = 3u; // -Y
            uv = vec2f(dir.x, -dir.z) / abs_dir.y;
        }
    } else {
        // Z axis is dominant
        if (dir.z > 0.0) {
            face = 4u; // +Z
            uv = vec2f(dir.x, -dir.y) / dir.z;
        } else {
            face = 5u; // -Z
            uv = vec2f(-dir.x, -dir.y) / abs_dir.z;
        }
    }
    
    // Convert from [-1,1] to [0,1]
    uv = uv * 0.5 + 0.5;
    return vec3u(u32(uv.x * 511.999), u32(uv.y * 511.999), face);
}

// Sample an environment map
fn sample_environment(dir: vec3f, level: f32) -> vec4f {
    let cubemap_result = dir_to_cube_uv(dir);
    let uv = vec2f(cubemap_result.xy) / 512.0 + 0.5 / 512.0;
    return textureSampleLevel(input_texture, input_sampler, uv, cubemap_result.z, level);
}

// Calculate tangent space for the given normal
fn calculate_tangent_frame(normal: vec3f) -> mat3x3f {
    // Use a robust method to pick a tangent
    var up = vec3f(1.0, 0.0, 0.0);
    if abs(normal.z) < 0.999 {
        up = vec3f(0.0, 0.0, 1.0);
    }
    let tangent = normalize(cross(up, normal));
    let bitangent = cross(normal, tangent);
    return mat3x3f(tangent, bitangent, normal);
}

// Sample GGX distribution to get a microfacet normal
fn sample_ggx(roughness: f32, xi: vec2f) -> vec3f {
    let a = roughness * roughness;
    
    // Sample spherical angle and azimuth
    let phi = 2.0 * PI * xi.x;
    let cos_theta = sqrt((1.0 - xi.y) / (1.0 + (a * a - 1.0) * xi.y));
    let sin_theta = sqrt(1.0 - cos_theta * cos_theta);
    
    // Convert to cartesian
    return vec3f(
        sin_theta * cos(phi),
        sin_theta * sin(phi),
        cos_theta
    );
}

// Hammersley sequence for quasi-random points
fn hammersley_2d(i: u32, n: u32) -> vec2f {
    // Van der Corput sequence
    var bits = i;
    bits = (bits << 16u) | (bits >> 16u);
    bits = ((bits & 0x55555555u) << 1u) | ((bits & 0xAAAAAAAAu) >> 1u);
    bits = ((bits & 0x33333333u) << 2u) | ((bits & 0xCCCCCCCCu) >> 2u);
    bits = ((bits & 0x0F0F0F0Fu) << 4u) | ((bits & 0xF0F0F0F0u) >> 4u);
    bits = ((bits & 0x00FF00FFu) << 8u) | ((bits & 0xFF00FF00u) >> 8u);
    let vdc = f32(bits) * 2.3283064365386963e-10; // 1 / 0x100000000
    return vec2f(f32(i) / f32(n), vdc);
}

@compute
@workgroup_size(8, 8, 1)
fn generate_radiance_map(@builtin(global_invocation_id) global_id: vec3u) {
    let size = textureDimensions(output_texture).xy;
    let invSize = 1.0 / vec2f(size);
    
    let coords = vec2u(global_id.xy);
    let face = global_id.z;
    
    if (any(coords >= size)) {
        return;
    }
    
    // Convert texture coordinates to direction vector
    let uv = (vec2f(coords) + 0.5) * invSize;
    let N = sample_cube_dir(uv, face);
    
    // Create tangent space matrix
    let tangent_frame = calculate_tangent_frame(N);
    
    // Calculate roughness from mip level or from constant roughness
    let roughness = constants.roughness;
    
    var radiance = vec3f(0.0);
    var total_weight = 0.0;
    
    for (var i = 0u; i < constants.sample_count; i++) {
        // Generate quasi-random sample
        let xi = hammersley_2d(i, constants.sample_count);
        
        // Sample GGX microfacet normal in tangent space
        let H = sample_ggx(roughness, xi);
        
        // Transform to world space
        let H_world = tangent_frame * H;
        
        // Calculate reflection direction from the normal and half vector
        let L = reflect(-N, H_world);
        
        // Make sure the reflection is in the hemisphere
        let NdotL = dot(N, L);
        
        if (NdotL > 0.0) {
            // Sample environment map in reflection direction
            // For base mip, use level 0, for other mips use filtered lower mips as source
            let level = select(0.0, constants.mip_level - 1.0, constants.mip_level > 0.0);
            let sample_color = sample_environment(L, level).rgb;
            
            // Add weighted sample to final radiance
            let weight = NdotL;
            radiance += sample_color * weight;
            total_weight += weight;
        }
    }
    
    // Normalize by total weight
    if total_weight > 0.0 {
        radiance = radiance / total_weight;
    } else {
        radiance = vec3f(0.0);
    }
    
    // Write result to output texture
    textureStore(output_texture, coords, face, vec4f(radiance, 1.0));
}

@compute
@workgroup_size(8, 8, 1)
fn generate_irradiance_map(@builtin(global_invocation_id) global_id: vec3u) {
    let size = textureDimensions(output_texture).xy;
    let invSize = 1.0 / vec2f(size);
    
    let coords = vec2u(global_id.xy);
    let face = global_id.z;
    
    if (any(coords >= size)) {
        return;
    }
    
    // Convert texture coordinates to direction vector
    let uv = (vec2f(coords) + 0.5) * invSize;
    let N = sample_cube_dir(uv, face);
    
    // Create tangent space matrix
    let tangent_frame = calculate_tangent_frame(N);
    
    var irradiance = vec3f(0.0);
    var total_weight = 0.0;
    
    // Use the sample count from constants instead of hardcoded value
    let sample_count = constants.sample_count;
    
    for (var i = 0u; i < sample_count; i++) {
        // Generate quasi-random sample on hemisphere
        let xi = hammersley_2d(i, sample_count);
        
        // Cosine weighted hemisphere sampling
        let phi = 2.0 * PI * xi.x;
        let cos_theta = sqrt(xi.y);
        let sin_theta = sqrt(1.0 - cos_theta * cos_theta);
        
        // Spherical to cartesian in tangent space
        let tangent_sample = vec3f(
            sin_theta * cos(phi),
            sin_theta * sin(phi),
            cos_theta
        );
        
        // Transform to world space
        let L = tangent_frame * tangent_sample;
        
        // Sample environment with low mip level to reduce noise
        let sample_color = sample_environment(L, 0.0).rgb;
        
        // Cosine weighting is already in the distribution
        let weight = cos_theta * sin_theta;
        irradiance += sample_color * weight;
        total_weight += weight;
    }
    
    // Normalize and scale by PI
    if total_weight > 0.0 {
        irradiance = irradiance / total_weight * PI;
    } else {
        irradiance = vec3f(0.0);
    }
    
    // Write result to output texture
    textureStore(output_texture, coords, face, vec4f(irradiance, 1.0));
}

// Workgroup size 8x8

// 128x128 - 16x16 dispatch
// 64x64 - 8x8 dispatch
// 32x32 - 4x4 dispatch
// 16x16 - 2x2 dispatch
// 8x8 - 1x1 dispatch

// 4x4 -]
// 2x2 -]>  1x1 dispatch
// 1x1 -]

// 16 + 8 + 4 + 2 + 1 + 1 = 32x32 workgroups total


// # SPD:
// # Dispatch with mips = log2(input_mip_0_resolution), inverse_input_size = 1.0 / input_mip_0_resolution
// # If input texture does not have `mips` amount of mips, copy mip 0 to new texture of the correct size
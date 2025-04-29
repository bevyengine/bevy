#import bevy_render::maths::{PI, PI_2};
#import bevy_pbr::lighting::perceptualRoughnessToRoughness;

struct PrefilterConstants {
    mip_level: f32,
    sample_count: u32,
    roughness: f32,
    blue_noise_size: vec2f,
}

@group(0) @binding(0) var input_texture: texture_2d_array<f32>;
@group(0) @binding(1) var input_sampler: sampler;
@group(0) @binding(2) var output_texture: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(3) var<uniform> constants: PrefilterConstants;
@group(0) @binding(4) var blue_noise_texture: texture_2d<f32>;

// Tonemapping functions to reduce fireflies
fn rcp(x: f32) -> f32 { return 1.0 / x; }
fn max3(x: vec3f) -> f32 { return max(x.r, max(x.g, x.b)); }
fn tonemap(color: vec3f) -> vec3f {
    return color / (color + vec3(5000.0));
}
fn reverse_tonemap(color: vec3f) -> vec3f {
    return 5000.0 * color / (vec3(1.0) - color);
}

// Predefined set of uniform directions
fn get_uniform_direction(index: u32) -> vec3f {
    var dir = vec3f(0.0);
    
    switch(index % 64u) {
        case 0u: { dir = vec3f(0.91593, -0.347884, 0.200123); }
        case 1u: { dir = vec3f(-0.244493, -0.710186, -0.660196); }
        case 2u: { dir = vec3f(-0.838322, 0.259442, 0.479484); }
        case 3u: { dir = vec3f(0.245473, 0.891464, -0.380835); }
        case 4u: { dir = vec3f(0.632533, -0.155099, 0.758846); }
        case 5u: { dir = vec3f(-0.20644, -0.973183, -0.101474); }
        case 6u: { dir = vec3f(-0.269471, 0.0483681, -0.961793); }
        case 7u: { dir = vec3f(0.143331, 0.973557, 0.177887); }
        case 8u: { dir = vec3f(0.725872, -0.086002, -0.682432); }
        case 9u: { dir = vec3f(-0.076835, -0.886014, 0.457249); }
        case 10u: { dir = vec3f(-0.913781, 0.0503775, -0.403071); }
        case 11u: { dir = vec3f(0.0159914, 0.676129, 0.73661); }
        case 12u: { dir = vec3f(0.992288, 0.00772121, -0.12371); }
        case 13u: { dir = vec3f(0.00641109, -0.177892, -0.984029); }
        case 14u: { dir = vec3f(-0.985566, -0.0665794, 0.155651); }
        case 15u: { dir = vec3f(-0.0700448, 0.706071, -0.704668); }
        case 16u: { dir = vec3f(0.89279, 0.117001, 0.435013); }
        case 17u: { dir = vec3f(0.142896, -0.893697, -0.425307); }
        case 18u: { dir = vec3f(-0.687174, -0.132142, 0.714374); }
        case 19u: { dir = vec3f(-0.217251, 0.965143, -0.145946); }
        case 20u: { dir = vec3f(0.108209, 0.0279573, 0.993735); }
        case 21u: { dir = vec3f(0.274912, -0.952168, 0.133416); }
        case 22u: { dir = vec3f(-0.653478, -0.211134, -0.726904); }
        case 23u: { dir = vec3f(-0.307126, 0.85749, 0.412777); }
        case 24u: { dir = vec3f(0.831999, 0.327845, -0.447543); }
        case 25u: { dir = vec3f(0.283463, -0.663772, 0.692138); }
        case 26u: { dir = vec3f(-0.893939, -0.415437, -0.168182); }
        case 27u: { dir = vec3f(-0.106605, 0.211719, 0.971499); }
        case 28u: { dir = vec3f(0.873146, 0.474611, 0.11118); }
        case 29u: { dir = vec3f(0.332658, -0.572825, -0.74914); }
        case 30u: { dir = vec3f(-0.781162, -0.487098, 0.390541); }
        case 31u: { dir = vec3f(-0.490404, 0.734038, -0.469779); }
        case 32u: { dir = vec3f(0.604084, 0.431641, 0.669902); }
        case 33u: { dir = vec3f(0.593065, -0.782314, -0.190417); }
        case 34u: { dir = vec3f(-0.244516, -0.197766, 0.949263); }
        case 35u: { dir = vec3f(-0.650394, 0.754372, 0.0889437); }
        case 36u: { dir = vec3f(0.468682, 0.430484, -0.771376); }
        case 37u: { dir = vec3f(0.647992, -0.666677, 0.368305); }
        case 38u: { dir = vec3f(-0.604909, -0.626104, -0.492015); }
        case 39u: { dir = vec3f(-0.564322, 0.511928, 0.647666); }
        case 40u: { dir = vec3f(0.633455, 0.743985, -0.212653); }
        case 41u: { dir = vec3f(0.292272, -0.234942, 0.927027); }
        case 42u: { dir = vec3f(-0.600382, -0.796926, 0.0667077); }
        case 43u: { dir = vec3f(-0.497216, 0.350652, -0.793612); }
        case 44u: { dir = vec3f(0.516356, 0.783334, 0.346069); }
        case 45u: { dir = vec3f(0.729109, -0.451604, -0.514251); }
        case 46u: { dir = vec3f(-0.389822, -0.675926, 0.62543); }
        case 47u: { dir = vec3f(-0.856868, 0.458917, -0.234889); }
        case 48u: { dir = vec3f(0.189162, 0.381537, 0.904791); }
        case 49u: { dir = vec3f(0.907219, -0.4183, 0.0444719); }
        case 50u: { dir = vec3f(-0.225508, -0.532484, -0.815848); }
        case 51u: { dir = vec3f(-0.882371, 0.341401, 0.323833); }
        case 52u: { dir = vec3f(0.279638, 0.796232, -0.536486); }
        case 53u: { dir = vec3f(0.759697, -0.242935, 0.603194); }
        case 54u: { dir = vec3f(-0.265275, -0.929255, -0.257125); }
        case 55u: { dir = vec3f(-0.455978, 0.114803, 0.882555); }
        case 56u: { dir = vec3f(0.213508, 0.976688, 0.0222359); }
        case 57u: { dir = vec3f(0.536034, -0.10141, -0.838084); }
        case 58u: { dir = vec3f(-0.147707, -0.941925, 0.301597); }
        case 59u: { dir = vec3f(-0.822975, 0.102675, -0.558722); }
        case 60u: { dir = vec3f(0.0753368, 0.810439, 0.580958); }
        case 61u: { dir = vec3f(0.958193, -0.0618399, -0.279361); }
        case 62u: { dir = vec3f(-0.0168296, -0.509477, 0.860319); }
        case 63u: { dir = vec3f(-0.999999, 0.00159255, 0.0); }
        default: { dir = vec3f(0.0, 0.0, 1.0); }
    }
    
    return normalize(dir);
}

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
struct CubeUV {
    uv: vec2f,
    face: u32,
}
fn dir_to_cube_uv(dir: vec3f) -> CubeUV {
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
    return CubeUV(uv * 0.5 + 0.5, face);
}

// Sample an environment map with a specific LOD
fn sample_environment(dir: vec3f, level: f32) -> vec4f {
    let cube_uv = dir_to_cube_uv(dir);
    return textureSampleLevel(input_texture, input_sampler, cube_uv.uv, cube_uv.face, level);
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

// Blue noise randomization
fn blue_noise_offset(pixel_coords: vec2u) -> vec2f {
    // Get a stable random offset for this pixel
    let noise_size = vec2u(u32(constants.blue_noise_size.x), u32(constants.blue_noise_size.y));
    let noise_coords = pixel_coords % noise_size;
    return textureSampleLevel(blue_noise_texture, input_sampler, vec2f(noise_coords) / constants.blue_noise_size, 0.0).rg;
}

// GGX/Trowbridge-Reitz normal distribution function (D term)
fn D_GGX(roughness: f32, NdotH: f32) -> f32 {
    let oneMinusNdotHSquared = 1.0 - NdotH * NdotH;
    let a = NdotH * roughness;
    let k = roughness / (oneMinusNdotHSquared + a * a);
    let d = k * k * (1.0 / PI);
    return d;
}

// Importance sample GGX normal distribution function for a given roughness
fn importance_sample_ggx(xi: vec2f, roughness: f32, normal: vec3f) -> vec3f {
    // Use roughness^2 to ensure correct specular highlights
    let a = roughness * roughness;
    
    // Sample in spherical coordinates
    let phi = 2.0 * PI * xi.x;
    
    // GGX mapping from uniform random to GGX distribution
    let cos_theta = sqrt((1.0 - xi.y) / (1.0 + (a * a - 1.0) * xi.y));
    let sin_theta = sqrt(1.0 - cos_theta * cos_theta);
    
    // Convert to cartesian
    let h = vec3f(
        sin_theta * cos(phi),
        sin_theta * sin(phi),
        cos_theta
    );
    
    // Transform from tangent to world space
    return calculate_tangent_frame(normal) * h;
}

// Calculate LOD for environment map lookup using filtered importance sampling
fn calculate_environment_map_lod(pdf: f32, width: f32, samples: f32) -> f32 {
    // Solid angle of current sample
    let omega_s = 1.0 / (samples * pdf);
    
    // Solid angle of a texel in the environment map
    let omega_p = 4.0 * PI / (6.0 * width * width);
    
    // Filtered importance sampling: compute the correct LOD
    return 0.5 * log2(omega_s / omega_p);
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
    let normal = sample_cube_dir(uv, face);
    
    // For radiance map, view direction = normal for perfect reflection
    let view = normal;
    
    // Get the roughness parameter
    let roughness = constants.roughness;
    
    // Get blue noise offset for stratification
    let blue_noise = blue_noise_offset(coords);
    
    var radiance = vec3f(0.0);
    var total_weight = 0.0;
    
    // Skip sampling for mirror reflection (roughness = 0)
    if (roughness < 0.01) {
        radiance = sample_environment(normal, 0.0).rgb;
        textureStore(output_texture, coords, face, vec4f(radiance, 1.0));
        return;
    }
    
    // For higher roughness values, use importance sampling
    let sample_count = constants.sample_count;
    
    for (var i = 0u; i < sample_count; i++) {
        // Get sample coordinates from Hammersley sequence with blue noise offset
        var xi = hammersley_2d(i, sample_count);
        xi = fract(xi + blue_noise); // Apply Cranley-Patterson rotation
        
        // Sample the GGX distribution to get a half vector
        let half_vector = importance_sample_ggx(xi, roughness, normal);
        
        // Calculate reflection vector from half vector
        let light_dir = reflect(-view, half_vector);
        
        // Calculate weight (N·L)
        let NoL = dot(normal, light_dir);
        
        if (NoL > 0.0) {
            // Calculate values needed for PDF
            let NoH = dot(normal, half_vector);
            let VoH = dot(view, half_vector);
            
            // Probability Distribution Function
            let pdf = D_GGX(roughness, NoH) * NoH / (4.0 * VoH);
            
            // Calculate LOD using filtered importance sampling
            // This is crucial to avoid fireflies and improve quality
            let width = f32(size.x);
            let lod = calculate_environment_map_lod(pdf, width, f32(sample_count));
            
            // Get source mip level - ensure we don't go negative
            let source_mip = max(0.0, lod);
            
            // Sample environment map with the light direction
            var sample_color = sample_environment(light_dir, source_mip).rgb;
            sample_color = tonemap(sample_color);
            
            // Accumulate weighted sample
            radiance += sample_color * NoL;
            total_weight += NoL;
        }
    }
    
    // Normalize by total weight
    if (total_weight > 0.0) {
        radiance = radiance / total_weight;
    }

    // Reverse tonemap
    radiance = reverse_tonemap(radiance);
    
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
    let normal = sample_cube_dir(uv, face);
    
    // Create tangent space matrix
    let tangent_frame = calculate_tangent_frame(normal);
    
    // Get blue noise offset for stratification
    let blue_noise = blue_noise_offset(coords);
    
    var irradiance = vec3f(0.0);
    var total_weight = 0.0;
    
    let sample_count = min(constants.sample_count, 64u);
    
    for (var i = 0u; i < sample_count; i++) {
        // Using a predefined set of directions provides good hemisphere coverage for diffuse
        var dir = get_uniform_direction((i + u32(coords.x * 7u + coords.y * 11u + face * 5u)) % 64u);
        
        // Ensure the direction is in the hemisphere defined by the normal
        let NoL = dot(normal, dir);
        
        // Flip the direction if it's in the wrong hemisphere
        if (NoL < 0.0) {
            dir = -dir;
        }
        
        // Recalculate NoL after possible flipping
        let weight = max(dot(normal, dir), 0.0);
        
        if (weight > 0.0) {
            // Lambert PDF
            let pdf = weight / PI;
            let width = f32(size.x);

            // Filtered importance sampling
            let mip_level = clamp(
                calculate_environment_map_lod(pdf, width, f32(sample_count)),
                1.0, 
                constants.roughness * 3.0
            );
            
            // Sample environment with the calculated mip level
            let sample_color = sample_environment(dir, mip_level).rgb;
            
            // Accumulate the sample
            irradiance += sample_color * weight;
            total_weight += weight;
        }
    }
    
    // Normalize and scale by PI for diffuse BRDF
    if (total_weight > 0.0) {
        irradiance = irradiance / total_weight * PI;
    }
    
    // Add some low-frequency ambient term to avoid completely dark areas
    irradiance = max(irradiance, vec3f(0.01, 0.01, 0.01));
    
    // Write result to output texture
    textureStore(output_texture, coords, face, vec4f(irradiance, 1.0));
}

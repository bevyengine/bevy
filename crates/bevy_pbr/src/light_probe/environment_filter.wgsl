#import bevy_render::maths::PI
#import bevy_pbr::{
    lighting,
    utils::{sample_cosine_hemisphere, dir_to_cube_uv, sample_cube_dir, hammersley_2d, rand_vec2f}
}

struct FilteringConstants {
    mip_level: f32,
    sample_count: u32,
    roughness: f32,
    noise_size_bits: vec2u,
}

@group(0) @binding(0) var input_texture: texture_2d_array<f32>;
@group(0) @binding(1) var input_sampler: sampler;
@group(0) @binding(2) var output_texture: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(3) var<uniform> constants: FilteringConstants;
@group(0) @binding(4) var blue_noise_texture: texture_2d_array<f32>;

// Sample an environment map with a specific LOD
fn sample_environment(dir: vec3f, level: f32) -> vec4f {
    let cube_uv = dir_to_cube_uv(dir);
    return textureSampleLevel(input_texture, input_sampler, cube_uv.uv, cube_uv.face, level);
}

// Blue noise randomization
#ifdef HAS_BLUE_NOISE
fn sample_noise(pixel_coords: vec2u) -> vec4f {
    let noise_size = vec2u(1) << constants.noise_size_bits;
    let noise_size_mask = noise_size - vec2u(1u);
    let noise_coords = pixel_coords & noise_size_mask;
    let uv = vec2f(noise_coords) / vec2f(noise_size);
    return textureSampleLevel(blue_noise_texture, input_sampler, uv, 0u, 0.0);
}
#else
// pseudo-random numbers using RNG
fn sample_noise(pixel_coords: vec2u) -> vec4f {
    var rng_state: u32 = (pixel_coords.x * 3966231743u) ^ (pixel_coords.y * 3928936651u);
    let rnd = rand_vec2f(&rng_state);
    return vec4f(rnd, 0.0, 0.0);
}
#endif

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
    
    // Convert perceptual roughness to physical microfacet roughness
    let perceptual_roughness = constants.roughness;
    let roughness = lighting::perceptualRoughnessToRoughness(perceptual_roughness);
    
    // Get blue noise offset for stratification
    let vector_noise = sample_noise(coords);
    
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
        xi = fract(xi + vector_noise.rg); // Apply Cranley-Patterson rotation
        
        // Sample the GGX distribution with the spherical-cap VNDF method
        let light_dir = lighting::sample_visible_ggx(xi, roughness, normal, view);
        
        // Calculate weight (N·L)
        let NdotL = dot(normal, light_dir);
        
        if (NdotL > 0.0) {
            // Reconstruct the microfacet half-vector from view and light and compute PDF terms
            let half_vector = normalize(view + light_dir);
            let NdotH = dot(normal, half_vector);
            let NdotV = dot(normal, view);
            
            // Get the geometric shadowing term
            let G = lighting::G_Smith(NdotV, NdotL, roughness);
            
            // PDF that matches the bounded-VNDF sampling
            let pdf = lighting::ggx_vndf_pdf(view, NdotH, roughness);
            
            // Calculate LOD using filtered importance sampling
            // This is crucial to avoid fireflies and improve quality
            let width = f32(size.x);
            let lod = calculate_environment_map_lod(pdf, width, f32(sample_count));
            
            // Get source mip level - ensure we don't go negative
            let source_mip = max(0.0, lod);
            
            // Sample environment map with the light direction
            var sample_color = sample_environment(light_dir, source_mip).rgb;
            
            // Accumulate weighted sample, including geometric term
            radiance += sample_color * NdotL * G;
            total_weight += NdotL * G;
        }
    }
    
    // Normalize by total weight
    if (total_weight > 0.0) {
        radiance = radiance / total_weight;
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
    let normal = sample_cube_dir(uv, face);
    
    var irradiance = vec3f(0.0);
    
    // Use uniform sampling on a hemisphere
    for (var i = 0u; i < constants.sample_count; i++) {
        // Build a deterministic RNG seed for this pixel / sample
        // 4 randomly chosen 32-bit primes
        var rng: u32 = (coords.x * 2131358057u) ^ (coords.y * 3416869721u) ^ (face * 1199786941u) ^ (i * 566200673u);

        // Sample a direction from the upper hemisphere around the normal
        var sample_dir = sample_cosine_hemisphere(normal, &rng);

        // Sample environment with level 0 (no mip)
        var sample_color = sample_environment(sample_dir, 0.0).rgb;
        
        // Accumulate the contribution
        irradiance += sample_color;
    }

    // Normalize by number of samples (cosine-weighted sampling already accounts for PDF)
    irradiance = irradiance / f32(constants.sample_count);
    
    // Write result to output texture
    textureStore(output_texture, coords, face, vec4f(irradiance, 1.0));
}

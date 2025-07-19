#import bevy_render::maths::{PI, PI_2, orthonormalize};
#import bevy_pbr::utils::rand_f;

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
@group(0) @binding(4) var blue_noise_texture: texture_2d<f32>;

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

// Hammersley sequence for quasi-random points
fn hammersley_2d(i: u32, n: u32) -> vec2f {
    let inv_n = 1.0 / f32(n);
    let vdc = f32(reverseBits(i)) * 2.3283064365386963e-10; // 1/2^32
    return vec2f(f32(i) * inv_n, vdc);
}

// Blue noise randomization
fn sample_noise(pixel_coords: vec2u) -> vec4f {
    let noise_size = vec2u(1) << constants.noise_size_bits;
    let noise_size_mask = noise_size - vec2u(1u);
    let noise_coords = pixel_coords & noise_size_mask;
    let uv = vec2f(noise_coords) / vec2f(noise_size);
    return textureSampleLevel(blue_noise_texture, input_sampler, uv, 0.0);
}

// from bevy_pbr/src/render/pbr_lighting.wgsl
fn perceptualRoughnessToRoughness(perceptualRoughness: f32) -> f32 {
    // clamp perceptual roughness to prevent precision problems
    // According to Filament design 0.089 is recommended for mobile
    // Filament uses 0.045 for non-mobile
    let clampedPerceptualRoughness = clamp(perceptualRoughness, 0.089, 1.0);
    return clampedPerceptualRoughness * clampedPerceptualRoughness;
}

// GGX/Trowbridge-Reitz normal distribution function (D term)
// from bevy_pbr/src/render/pbr_lighting.wgsl
fn D_GGX(roughness: f32, NdotH: f32) -> f32 {
    let oneMinusNdotHSquared = 1.0 - NdotH * NdotH;
    let a = NdotH * roughness;
    let k = roughness / (oneMinusNdotHSquared + a * a);
    let d = k * k * (1.0 / PI);
    return d;
}

// Probability-density function that matches the bounded VNDF sampler (Listing 2)
fn ggx_vndf_pdf(i: vec3<f32>, NdotH: f32, roughness: f32) -> f32 {
    let ndf = D_GGX(roughness, NdotH);

    // Common terms
    let ai = roughness * i.xy;
    let len2 = dot(ai, ai);
    let t = sqrt(len2 + i.z * i.z);

    if (i.z >= 0.0) {
        let a = clamp(roughness, 0.0, 1.0);
        let s = 1.0 + length(i.xy);
        let a2 = a * a;
        let s2 = s * s;
        let k = (1.0 - a2) * s2 / (s2 + a2 * i.z * i.z);
        return ndf / (2.0 * (k * i.z + t));
    }

    // Backfacing case
    return ndf * (t - i.z) / (2.0 * len2);
}

// https://gpuopen.com/download/Bounded_VNDF_Sampling_for_Smith-GGX_Reflections.pdf
fn sample_visible_ggx(
    xi: vec2<f32>,
    roughness: f32,
    normal: vec3<f32>,
    view: vec3<f32>,
) -> vec3<f32> {
    let n = normal;
    let alpha = roughness;

    // Decompose view into components parallel/perpendicular to the normal
    let wi_n = dot(view, n);
    let wi_z = -n * wi_n;
    let wi_xy = view + wi_z;

    // Warp view vector to the unit-roughness configuration
    let wi_std = -normalize(alpha * wi_xy + wi_z);

    // Compute wi_std.z once for reuse
    let wi_std_z = dot(wi_std, n);

    // Bounded VNDF sampling
    // Compute the bound parameter k (Eq. 5) and the scaled z–limit b (Eq. 6)
    let s = 1.0 + length(wi_xy);
    let a = clamp(alpha, 0.0, 1.0);
    let a2 = a * a;
    let s2 = s * s;
    let k = (1.0 - a2) * s2 / (s2 + a2 * wi_n * wi_n);
    let b = select(wi_std_z, k * wi_std_z, wi_n > 0.0);

    // Sample a spherical cap in (-b, 1]
    let z = 1.0 - xi.y * (1.0 + b);
    let sin_theta = sqrt(max(0.0, 1.0 - z * z));
    let phi = 2.0 * PI * xi.x - PI;
    let x = sin_theta * cos(phi);
    let y = sin_theta * sin(phi);
    let c_std = vec3f(x, y, z);

    // Reflect the sample so that the normal aligns with +Z
    let up = vec3f(0.0, 0.0, 1.0);
    let wr = n + up;
    let c = dot(wr, c_std) * wr / wr.z - c_std;

    // Half-vector in the standard frame
    let wm_std = c + wi_std;
    let wm_std_z = n * dot(n, wm_std);
    let wm_std_xy = wm_std_z - wm_std;

    // Unwarp back to original roughness and compute microfacet normal
    let H = normalize(alpha * wm_std_xy + wm_std_z);

    // Reflect view to obtain the outgoing (light) direction
    return reflect(-view, H);
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

// Smith geometric shadowing function
fn G_Smith(NoV: f32, NoL: f32, roughness: f32) -> f32 {
    let k = roughness / 2.0;
    let GGXL = NoL / (NoL * (1.0 - k) + k);
    let GGXV = NoV / (NoV * (1.0 - k) + k);
    return GGXL * GGXV;
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
    let roughness = perceptualRoughnessToRoughness(perceptual_roughness);
    
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
        let light_dir = sample_visible_ggx(xi, roughness, normal, view);
        
        // Calculate weight (N·L)
        let NoL = dot(normal, light_dir);
        
        if (NoL > 0.0) {
            // Reconstruct the microfacet half-vector from view and light and compute PDF terms
            let half_vector = normalize(view + light_dir);
            let NoH = dot(normal, half_vector);
            let VoH = dot(view, half_vector);
            let NoV = dot(normal, view);
            
            // Get the geometric shadowing term
            let G = G_Smith(NoV, NoL, roughness);
            
            // PDF that matches the bounded-VNDF sampling
            let pdf = ggx_vndf_pdf(view, NoH, roughness);
            
            // Calculate LOD using filtered importance sampling
            // This is crucial to avoid fireflies and improve quality
            let width = f32(size.x);
            let lod = calculate_environment_map_lod(pdf, width, f32(sample_count));
            
            // Get source mip level - ensure we don't go negative
            let source_mip = max(0.0, lod);
            
            // Sample environment map with the light direction
            var sample_color = sample_environment(light_dir, source_mip).rgb;
            
            // Accumulate weighted sample, including geometric term
            radiance += sample_color * NoL * G;
            total_weight += NoL * G;
        }
    }
    
    // Normalize by total weight
    if (total_weight > 0.0) {
        radiance = radiance / total_weight;
    }
    
    // Write result to output texture
    textureStore(output_texture, coords, face, vec4f(radiance, 1.0));
}

// Calculate spherical coordinates using spiral pattern
// and golden angle to get a uniform distribution
fn uniform_sample_sphere(i: u32, normal: vec3f) -> vec3f {
    // Get stratified sample index
    let index = i % constants.sample_count;
    
    let golden_angle = 2.4;
    let full_sphere = f32(constants.sample_count) * 2.0;
    let z = 1.0 - (2.0 * f32(index) + 1.0) / full_sphere;
    let r = sqrt(1.0 - z * z);
    
    let phi = f32(index) * golden_angle;
    
    // Create the direction vector
    let dir_uniform = vec3f(
        r * cos(phi),
        r * sin(phi),
        z
    );

    let tangent_frame = orthonormalize(normal);
    return normalize(tangent_frame * dir_uniform);
}

// from bevy_solari/src/scene/sampling.wgsl
fn sample_cosine_hemisphere(normal: vec3<f32>, rng: ptr<function, u32>) -> vec3<f32> {
    let cos_theta = 1.0 - 2.0 * rand_f(rng);
    let phi = PI_2 * rand_f(rng);
    let sin_theta = sqrt(max(1.0 - cos_theta * cos_theta, 0.0));
    let x = normal.x + sin_theta * cos(phi);
    let y = normal.y + sin_theta * sin(phi);
    let z = normal.z + cos_theta;
    return vec3(x, y, z);
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
        var rng: u32 = (coords.x * 73856093u) ^ (coords.y * 19349663u) ^ (face * 83492791u) ^ i;

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

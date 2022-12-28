#define_import_path bevy_pbr::environment_map

@group(3) @binding(0)
var environment_map_brdf_lut: texture_2d<f32>;
@group(3) @binding(1)
var environment_map_diffuse: texture_cube<f32>;
@group(3) @binding(2)
var environment_map_specular: texture_cube<f32>;
@group(3) @binding(3)
var environment_map_sampler: sampler;

fn environment_map_diffuse(N: vec3<f32>, diffuse_color: vec3<f32>) -> vec3<f32> {
    let irradiance = textureSample(environment_map_diffuse, environment_map_sampler, N).rgb;
    return diffuse_color * irradiance;
}

fn environment_map_specular(NdotV: f32, perceptual_roughness: f32, R: vec3<f32>, F0: vec3<f32>) -> vec3<f32> {
    let environment_map_specular_mip_count = 5.0;
    let mip_level = perceptual_roughness * (environment_map_specular_mip_count - 1.0);

    let f_ab = textureSample(environment_map_brdf_lut, environment_map_sampler, vec2(NdotV, perceptual_roughness)).rg;
    let radiance = textureSampleLevel(environment_map_specular, environment_map_sampler, R, mip_level).rgb;
    let fss_ess = F0 * f_ab.x + f_ab.y;
    return fss_ess * radiance;
}

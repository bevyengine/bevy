#define_import_path bevy_pbr::environment_map

@group(3) @binding(0)
var environment_map_diffuse: texture_cube<f32>;
@group(3) @binding(1)
var environment_map_specular: texture_cube<f32>;
@group(3) @binding(2)
var environment_map_sampler: sampler;

struct EnvironmentMapLight {
    diffuse: vec3<f32>,
    specular: vec3<f32>,
};

fn environment_map_light(
    perceptual_roughness: f32,
    roughness: f32,
    diffuse_color: vec3<f32>,
    NdotV: f32,
    N: vec3<f32>,
    R: vec3<f32>,
    F0: vec3<f32>,
    clear_coat: f32,
    clear_coat_perceptual_roughness: f32,
    clear_coat_roughness: f32,
) -> EnvironmentMapLight {
    let environment_map_specular_smallest_mip_level = 10.0;

    // Split-sum approximation for image based lighting: https://cdn2.unrealengine.com/Resources/files/2013SiggraphPresentationsNotes-26915738.pdf
    let irradiance = textureSample(environment_map_diffuse, environment_map_sampler, N).rgb;
    let radiance = textureSampleLevel(environment_map_specular, environment_map_sampler, R, perceptual_roughness * environment_map_specular_smallest_mip_level).rgb;

    // Scale/bias approximation: https://www.unrealengine.com/en-US/blog/physically-based-shading-on-mobile
    let c0 = vec4(-1.0, -0.0275, -0.572, 0.022);
    let c1 = vec4(1.0, 0.0425, 1.04, -0.04);
    let r = perceptual_roughness * c0 + c1;
    let a004 = min(r.x * r.x, exp2(-9.28 * NdotV)) * r.x + r.y;
    let f_ab = vec2(-1.04, 1.04) * a004 + r.zw;

    // Multiscattering approximation: https://www.jcgt.org/published/0008/01/03/paper.pdf
    // Useful reference: https://bruop.github.io/ibl
    let Fr = max(vec3(1.0 - roughness), F0) - F0;
    let kS = F0 + Fr * pow(1.0 - NdotV, 5.0);
    let FssEss = kS * f_ab.x + f_ab.y;
    let Ess = f_ab.x + f_ab.y;
    let Ems = 1.0 - Ess;
    let Favg = F0 + (1.0 - F0) / 21.0;
    let Fms = FssEss * Favg / (1.0 - Ems * Favg);
    let Edss = 1.0 - (FssEss + Fms * Ems);
    let kD = diffuse_color * Edss;

    var out: EnvironmentMapLight;
    out.diffuse = (Fms * Ems + kD) * irradiance;
    out.specular = FssEss * radiance;

    // Clear coat IBL: https://google.github.io/filament/Filament.html#lighting/imagebasedlights/clearcoat
    if clear_coat != 0.0 {
        let Fc = F_Schlick(0.04, 1.0, NdotV) * clear_coat;
        let Fr = max(vec3(1.0 - clear_coat_roughness), F0) - F0;
        let kS = F0 + Fr * pow(1.0 - NdotV, 5.0);
        let FssEss = kS * f_ab.x + f_ab.y;

        let attenuation = 1.0 - Fc;
        out.diffuse *= attenuation;
        out.specular *= attenuation * attenuation;

        let radiance = textureSampleLevel(environment_map_specular, environment_map_sampler, R, clear_coat_perceptual_roughness * environment_map_specular_smallest_mip_level).rgb;
        out.specular += FssEss * radiance;
    }

    return out;
}

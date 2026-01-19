#define_import_path bevy_solari::brdf

#import bevy_pbr::lighting::{F_AB, D_GGX, V_SmithGGXCorrelated, fresnel, specular_multiscatter}
#import bevy_pbr::pbr_functions::{calculate_diffuse_color, calculate_F0}
#import bevy_render::maths::PI
#import bevy_solari::scene_bindings::{ResolvedMaterial, MIRROR_ROUGHNESS_THRESHOLD}

fn evaluate_brdf(
    world_normal: vec3<f32>,
    wo: vec3<f32>,
    wi: vec3<f32>,
    material: ResolvedMaterial,
) -> vec3<f32> {
    let diffuse_brdf = evaluate_diffuse_brdf(world_normal, wi, material.base_color, material.metallic);
    let specular_brdf = evaluate_specular_brdf(
        world_normal,
        wo,
        wi,
        material.base_color,
        material.metallic,
        material.reflectance,
        material.perceptual_roughness,
        material.roughness,
    );
    return diffuse_brdf + specular_brdf;
}

fn evaluate_diffuse_brdf(N: vec3<f32>, L: vec3<f32>, base_color: vec3<f32>, metallic: f32) -> vec3<f32> {
    let diffuse_color = calculate_diffuse_color(base_color, metallic, 0.0, 0.0) / PI;
    return diffuse_color * saturate(dot(N, L));
}

fn evaluate_specular_brdf(
    N: vec3<f32>,
    V: vec3<f32>,
    L: vec3<f32>,
    base_color: vec3<f32>,
    metallic: f32,
    reflectance: vec3<f32>,
    perceptual_roughness: f32,
    roughness: f32,
) -> vec3<f32> {
    let H = normalize(L + V);
    let NdotL = saturate(dot(N, L));
    let NdotH = saturate(dot(N, H));
    let LdotH = saturate(dot(L, H));
    let NdotV = max(dot(N, V), 0.0001);

    let F0 = calculate_F0(base_color, metallic, reflectance);
    let F = fresnel(F0, LdotH);

    if roughness <= MIRROR_ROUGHNESS_THRESHOLD {
        return F;
    }

    let D = D_GGX(roughness, NdotH);
    let Vs = V_SmithGGXCorrelated(roughness, NdotV, NdotL);
    let F_ab = F_AB(perceptual_roughness, NdotV);
    return specular_multiscatter(D, Vs, F, F0, F_ab, 1.0) * saturate(dot(N, L));
}

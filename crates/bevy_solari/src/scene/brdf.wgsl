enable wgpu_ray_query;

#define_import_path bevy_solari::brdf

#import bevy_core_pipeline::tonemapping::tonemapping_luminance as luminance
#import bevy_pbr::lighting::{D_GGX, V_SmithGGXCorrelated, specular_multiscatter}
#import bevy_pbr::pbr_functions::calculate_F0_dielectric
#import bevy_pbr::utils::{rand_f, sample_cosine_hemisphere}
#import bevy_render::maths::{PI, orthonormalize}
#import bevy_solari::sampling::{sample_ggx_vndf, ggx_vndf_pdf, ggx_vndf_sample_invalid}
#import bevy_solari::scene_bindings::{ResolvedMaterial, MIRROR_ROUGHNESS_THRESHOLD, brdf_dfg_lut, brdf_dfg_lut_sampler}

struct EvaluateAndSampleBrdfResult {
    wi: vec3<f32>,
    throughput: vec3<f32>,
    pdf: f32,
}

struct LobeReflectances {
    specular: vec3<f32>,
    diffuse: vec3<f32>,
}

// Hemispherical reflectance of each lobe
fn lobe_reflectances(F0_metal: vec3<f32>, F0_dielectric: vec3<f32>, material: ResolvedMaterial, F_ab: vec2<f32>) -> LobeReflectances {
    let multiscattering_factor = 1.0 / (F_ab.x + F_ab.y) - 1.0;
    let rho_specular_metallic = (F0_metal * F_ab.x + F_ab.y) * (1.0 + F0_metal * multiscattering_factor);
    let rho_specular_dielectric = (F0_dielectric * F_ab.x + F_ab.y) * (1.0 + F0_dielectric * multiscattering_factor);
    return LobeReflectances(
        mix(rho_specular_dielectric, rho_specular_metallic, material.metallic),
        (1.0 - material.metallic) * (1.0 - rho_specular_dielectric) * material.base_color,
    );
}

fn evaluate_and_sample_brdf(
    wo: vec3<f32>,
    world_normal: vec3<f32>,
    material: ResolvedMaterial,
    F_ab: vec2<f32>,
    rng: ptr<function, u32>,
) -> EvaluateAndSampleBrdfResult {
    let NdotV = dot(world_normal, wo);
    if NdotV < 0.0001 { return EvaluateAndSampleBrdfResult(vec3(0.0), vec3(0.0), 0.0); }
    let F0_metal = material.base_color;
    let F0_dielectric = calculate_F0_dielectric(vec3(material.reflectance));
    let rho = lobe_reflectances(F0_metal, F0_dielectric, material, F_ab);
    let specular_weight = luminance(rho.specular) / luminance(rho.specular + rho.diffuse);
    let diffuse_weight = 1.0 - specular_weight;

    let TBN = orthonormalize(world_normal);
    let T = TBN[0];
    let B = TBN[1];
    let N = TBN[2];

    let wo_tangent = vec3(dot(wo, T), dot(wo, B), dot(wo, N));

    var wi: vec3<f32>;
    var wi_tangent: vec3<f32>;
    let diffuse_selected = rand_f(rng) < diffuse_weight;
    if diffuse_selected {
        wi = sample_cosine_hemisphere(world_normal, rng);
        wi_tangent = vec3(dot(wi, T), dot(wi, B), dot(wi, N));
    } else {
        wi_tangent = sample_ggx_vndf(wo_tangent, material.roughness, rng);
        if ggx_vndf_sample_invalid(wi_tangent) {
            return EvaluateAndSampleBrdfResult(vec3(0.0), vec3(0.0), 0.0);
        }
        wi = wi_tangent.x * T + wi_tangent.y * B + wi_tangent.z * N;

        // Mirror specular is a delta function
        if material.roughness <= MIRROR_ROUGHNESS_THRESHOLD {
            return EvaluateAndSampleBrdfResult(
                wi,
                evaluate_specular_brdf(wo, wi, world_normal, material, F_ab) / specular_weight,
                bitcast<f32>(0x7F800000u) // INF
            );
        }
    }

    let diffuse_pdf = wi_tangent.z / PI;
    let specular_pdf = ggx_vndf_pdf(wo_tangent, wi_tangent, material.roughness);
    let pdf = (diffuse_weight * diffuse_pdf) + (specular_weight * specular_pdf);
    let throughput = evaluate_brdf(wo, wi, world_normal, material, F_ab) / pdf;
    return EvaluateAndSampleBrdfResult(wi, throughput, pdf);
}

fn evaluate_brdf(
    wo: vec3<f32>,
    wi: vec3<f32>,
    world_normal: vec3<f32>,
    material: ResolvedMaterial,
    F_ab: vec2<f32>,
) -> vec3<f32> {
    return evaluate_diffuse_brdf(wo, wi, world_normal, material, F_ab) + evaluate_specular_brdf(wo, wi, world_normal, material, F_ab);
}

fn evaluate_diffuse_brdf(wo: vec3<f32>, wi: vec3<f32>, world_normal: vec3<f32>, material: ResolvedMaterial, F_ab: vec2<f32>) -> vec3<f32> {
    let NdotL = dot(world_normal, wi);
    let NdotV = dot(world_normal, wo);
    if NdotL < 0.0001 || NdotV < 0.0001 { return vec3(0.0); }
    let F0_metal = material.base_color;
    let F0_dielectric = calculate_F0_dielectric(vec3(material.reflectance));
    let rho = lobe_reflectances(F0_metal, F0_dielectric, material, F_ab);
    return rho.diffuse / PI * NdotL;
}

fn evaluate_specular_brdf(wo: vec3<f32>, wi: vec3<f32>, world_normal: vec3<f32>, material: ResolvedMaterial, F_ab: vec2<f32>) -> vec3<f32> {
    let H = normalize(wi + wo);
    let NdotL = dot(world_normal, wi);
    let NdotH = dot(world_normal, H);
    let LdotH = dot(wi, H);
    let NdotV = dot(world_normal, wo);
    if NdotL < 0.0001 || NdotH < 0.0001 || LdotH < 0.0001 || NdotV < 0.0001 { return vec3(0.0); }

    let F0_metal = material.base_color;
    let F0_dielectric = calculate_F0_dielectric(vec3(material.reflectance));

    if material.roughness <= MIRROR_ROUGHNESS_THRESHOLD {
        if abs(NdotH - 1.0) < 0.0001 {
            let F_metal = fresnel(F0_metal, LdotH);
            let F_dielectric = fresnel(F0_dielectric, LdotH);
            return mix(F_dielectric, F_metal, material.metallic);
        } else {
            return vec3(0.0);
        }
    }

    let D = D_GGX(material.roughness, NdotH);
    let Vs = V_SmithGGXCorrelated(material.roughness, NdotV, NdotL);
    let F_metal = fresnel(F0_metal, LdotH);
    let F_dielectric = fresnel(F0_dielectric, LdotH);
    return mix(specular_multiscatter(D, Vs, F_dielectric, F0_dielectric, F_ab, 1.0),
               specular_multiscatter(D, Vs, F_metal, F0_metal, F_ab, 1.0),
               material.metallic) * NdotL;
}

fn brdf_pdf(wo: vec3<f32>, wi: vec3<f32>, world_normal: vec3<f32>, material: ResolvedMaterial, F_ab: vec2<f32>) -> f32 {
    let NdotV = max(dot(world_normal, wo), 0.0001);
    let F0_metal = material.base_color;
    let F0_dielectric = calculate_F0_dielectric(vec3(material.reflectance));
    let rho = lobe_reflectances(F0_metal, F0_dielectric, material, F_ab);
    let specular_weight = luminance(rho.specular) / luminance(rho.specular + rho.diffuse);
    let diffuse_weight = 1.0 - specular_weight;

    let TBN = orthonormalize(world_normal);
    let T = TBN[0];
    let B = TBN[1];
    let N = TBN[2];

    let wo_tangent = vec3(dot(wo, T), dot(wo, B), dot(wo, N));
    let wi_tangent = vec3(dot(wi, T), dot(wi, B), dot(wi, N));

    let diffuse_pdf = wi_tangent.z / PI;
    let specular_pdf = ggx_vndf_pdf(wo_tangent, wi_tangent, material.roughness);
    return (diffuse_weight * diffuse_pdf) + (specular_weight * specular_pdf);
}

fn fresnel(f0: vec3<f32>, LdotH: f32) -> vec3<f32> {
    return f0 + (1.0 - f0) * pow(1.0 - LdotH, 5.0);
}

// Scale/bias approximation
fn F_AB(perceptual_roughness: f32, NdotV: f32) -> vec2<f32> {
    return textureSampleLevel(brdf_dfg_lut, brdf_dfg_lut_sampler, vec2<f32>(NdotV, perceptual_roughness), 0.0).rg;
}

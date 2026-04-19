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
    rho_spec: vec3<f32>,
    rho_diff: vec3<f32>,
}

// Hemispherical reflectance of each lobe
fn lobe_reflectances(F0_metal: vec3<f32>, F0_dielectric: vec3<f32>, material: ResolvedMaterial, NdotV: f32) -> LobeReflectances {
    if material.roughness <= MIRROR_ROUGHNESS_THRESHOLD {
        let F_m = fresnel(F0_metal, NdotV);
        let F_d = fresnel(F0_dielectric, NdotV);
        return LobeReflectances(
            material.metallic * F_m + (1.0 - material.metallic) * F_d,
            (1.0 - material.metallic) * (vec3(1.0) - F_d) * material.base_color,
        );
    }
    let F_ab = F_AB(material.perceptual_roughness, NdotV);
    let ms_factor = 1.0 / (F_ab.x + F_ab.y) - 1.0;
    let rho_spec_m = (F0_metal * F_ab.x + vec3(F_ab.y)) * (vec3(1.0) + F0_metal * ms_factor);
    let rho_spec_d = (F0_dielectric * F_ab.x + vec3(F_ab.y)) * (vec3(1.0) + F0_dielectric * ms_factor);
    return LobeReflectances(
        material.metallic * rho_spec_m + (1.0 - material.metallic) * rho_spec_d,
        (1.0 - material.metallic) * (vec3(1.0) - rho_spec_d) * material.base_color,
    );
}

fn evaluate_and_sample_brdf(
    wo: vec3<f32>,
    world_normal: vec3<f32>,
    material: ResolvedMaterial,
    rng: ptr<function, u32>,
) -> EvaluateAndSampleBrdfResult {
    let TBN = orthonormalize(world_normal);
    let T = TBN[0];
    let B = TBN[1];
    let N = TBN[2];

    let NdotV = dot(N, wo);
    if NdotV < 0.0001 { return EvaluateAndSampleBrdfResult(vec3(0.0), vec3(0.0), 0.0); }

    let F0_metal = material.base_color;
    let F0_dielectric = calculate_F0_dielectric(vec3(material.reflectance));
    let rho = lobe_reflectances(F0_metal, F0_dielectric, material, NdotV);
    let specular_weight = luminance(rho.rho_spec) / luminance(rho.rho_spec + rho.rho_diff);

    let wo_tangent = vec3(dot(wo, T), dot(wo, B), dot(wo, N));
    var wi: vec3<f32>;
    var wi_tangent: vec3<f32>;
    let diffuse_selected = rand_f(rng) < (1.0 - specular_weight);
    if diffuse_selected {
        wi = sample_cosine_hemisphere(N, rng);
        wi_tangent = vec3(dot(wi, T), dot(wi, B), dot(wi, N));
    } else {
        wi_tangent = sample_ggx_vndf(wo_tangent, material.roughness, rng);
        if ggx_vndf_sample_invalid(wi_tangent) { return EvaluateAndSampleBrdfResult(vec3(0.0), vec3(0.0), 0.0); }
        wi = wi_tangent.x * T + wi_tangent.y * B + wi_tangent.z * N;
        
        // Mirror specular is a delta function
        if material.roughness <= MIRROR_ROUGHNESS_THRESHOLD {
            return EvaluateAndSampleBrdfResult(wi, rho.rho_spec / specular_weight, 1.0);
        }
    }

    let diffuse_pdf = wi_tangent.z / PI;
    let specular_pdf = ggx_vndf_pdf(wo_tangent, wi_tangent, material.roughness);
    let pdf = specular_weight * specular_pdf + (1.0 - specular_weight) * diffuse_pdf;
    let throughput = evaluate_brdf(wo, wi, world_normal, material) / pdf;
    return EvaluateAndSampleBrdfResult(wi, throughput, pdf);
}

fn evaluate_brdf(
    wo: vec3<f32>,
    wi: vec3<f32>,
    world_normal: vec3<f32>,
    material: ResolvedMaterial,
) -> vec3<f32> {
    return evaluate_diffuse_brdf(wo, wi, world_normal, material) + evaluate_specular_brdf(wo, wi, world_normal, material);
}

fn evaluate_diffuse_brdf(wo: vec3<f32>, wi: vec3<f32>, world_normal: vec3<f32>, material: ResolvedMaterial) -> vec3<f32> {
    let NdotL = dot(world_normal, wi);
    let NdotV = dot(world_normal, wo);
    if NdotL < 0.0001 || NdotV < 0.0001 { return vec3(0.0); }

    let F0_dielectric = calculate_F0_dielectric(vec3(material.reflectance));
    let rho = lobe_reflectances(material.base_color, F0_dielectric, material, NdotV);
    return rho.rho_diff / PI * NdotL;
}

fn evaluate_specular_brdf(wo: vec3<f32>, wi: vec3<f32>, world_normal: vec3<f32>, material: ResolvedMaterial) -> vec3<f32> {
    let H = normalize(wi + wo);
    let NdotL = dot(world_normal, wi);
    let NdotH = dot(world_normal, H);
    let HdotV = dot(H, wo);
    let NdotV = dot(world_normal, wo);
    if NdotL < 0.0001 || NdotH < 0.0001 || HdotV < 0.0001 || NdotV < 0.0001 { return vec3(0.0); }

    let F0_metal = material.base_color;
    let F0_dielectric = calculate_F0_dielectric(vec3(material.reflectance));

    if material.roughness <= MIRROR_ROUGHNESS_THRESHOLD {
        if abs(NdotH - 1.0) < 0.0001 {
            let F_m = fresnel(F0_metal, HdotV);
            let F_d = fresnel(F0_dielectric, HdotV);
            return material.metallic * F_m + (1.0 - material.metallic) * F_d;
        } else {
            return vec3(0.0);
        }
    }

    let D = D_GGX(material.roughness, NdotH);
    let Vs = V_SmithGGXCorrelated(material.roughness, NdotV, NdotL);
    let F_ab = F_AB(material.perceptual_roughness, NdotV);
    let F_m = fresnel(F0_metal, HdotV);
    let F_d = fresnel(F0_dielectric, HdotV);
    return (material.metallic * specular_multiscatter(D, Vs, F_m, F0_metal, F_ab, 1.0)
          + (1.0 - material.metallic) * specular_multiscatter(D, Vs, F_d, F0_dielectric, F_ab, 1.0)) * NdotL;
}

fn fresnel(f0: vec3<f32>, LdotH: f32) -> vec3<f32> {
    return f0 + (1.0 - f0) * pow(1.0 - LdotH, 5.0);
}

// Scale/bias approximation
fn F_AB(perceptual_roughness: f32, NdotV: f32) -> vec2<f32> {
    return textureSampleLevel(brdf_dfg_lut, brdf_dfg_lut_sampler, vec2<f32>(NdotV, perceptual_roughness), 0.0).rg;
}

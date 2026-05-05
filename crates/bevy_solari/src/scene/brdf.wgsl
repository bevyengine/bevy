enable wgpu_ray_query;

#define_import_path bevy_solari::brdf

#import bevy_core_pipeline::tonemapping::tonemapping_luminance as luminance
#import bevy_pbr::lighting::{D_GGX, V_SmithGGXCorrelated, specular_multiscatter}
#import bevy_pbr::pbr_functions::{calculate_diffuse_color, calculate_F0}
#import bevy_pbr::utils::{rand_f, sample_cosine_hemisphere}
#import bevy_render::maths::{PI, orthonormalize}
#import bevy_solari::sampling::{sample_ggx_vndf, ggx_vndf_pdf, ggx_vndf_sample_invalid}
#import bevy_solari::scene_bindings::{ResolvedMaterial, MIRROR_ROUGHNESS_THRESHOLD, brdf_dfg_lut, brdf_dfg_lut_sampler}

struct EvaluateAndSampleBrdfResult {
    wi: vec3<f32>,
    throughput: vec3<f32>,
    pdf: f32,
}

fn evaluate_and_sample_brdf(
    wo: vec3<f32>,
    world_normal: vec3<f32>,
    material: ResolvedMaterial,
    rng: ptr<function, u32>,
) -> EvaluateAndSampleBrdfResult {
    let NdotV = dot(world_normal, wo);
    if NdotV < 0.0001 { return EvaluateAndSampleBrdfResult(vec3(0.0), vec3(0.0), 0.0); }
    let F0 = calculate_F0(material.base_color, material.metallic, vec3(material.reflectance));
    let df = 1.0 - luminance(fresnel(F0, NdotV));

    let diffuse_weight = mix(df, 0.0, material.metallic);
    let specular_weight = 1.0 - diffuse_weight;

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
    }

    let diffuse_pdf = wi_tangent.z / PI;
    let specular_pdf = ggx_vndf_pdf(wo_tangent, wi_tangent, material.roughness);
    let pdf = (diffuse_weight * diffuse_pdf) + (specular_weight * specular_pdf);

    var throughput = evaluate_brdf(wo, wi, world_normal, material);
    if diffuse_selected || material.roughness > MIRROR_ROUGHNESS_THRESHOLD {
        throughput /= pdf;
    } else {
        throughput /= specular_weight;
    }

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
    let diffuse_color = calculate_diffuse_color(material.base_color, material.metallic, 0.0, 0.0) / PI;

    let NdotL = dot(world_normal, wi);
    let NdotV = dot(world_normal, wo);
    if NdotL < 0.0001 || NdotV < 0.0001 { return vec3(0.0); }
    let F0 = calculate_F0(material.base_color, material.metallic, vec3(material.reflectance));
    let layering = (1.0 - fresnel(F0, NdotL)) * (1.0 - fresnel(F0, NdotV));

    return diffuse_color * layering * NdotL;
}

fn evaluate_specular_brdf(wo: vec3<f32>, wi: vec3<f32>, world_normal: vec3<f32>, material: ResolvedMaterial) -> vec3<f32> {
    let H = normalize(wi + wo);
    let NdotL = dot(world_normal, wi);
    let NdotH = dot(world_normal, H);
    let LdotH = dot(wi, H);
    let NdotV = dot(world_normal, wo);
    if NdotL < 0.0001 || NdotH < 0.0001 || LdotH < 0.0001 || NdotV < 0.0001 { return vec3(0.0); }

    let F0 = calculate_F0(material.base_color, material.metallic, vec3(material.reflectance));
    let F = fresnel(F0, LdotH);

    if material.roughness <= MIRROR_ROUGHNESS_THRESHOLD {
        if abs(NdotH - 1.0) < 0.0001 {
            return F;
        } else {
            return vec3(0.0);
        }
    }

    let D = D_GGX(material.roughness, NdotH);
    let Vs = V_SmithGGXCorrelated(material.roughness, NdotV, NdotL);
    let F_ab = F_AB(material.perceptual_roughness, NdotV);
    return specular_multiscatter(D, Vs, F, F0, F_ab, 1.0) * NdotL;
}

fn fresnel(f0: vec3<f32>, LdotH: f32) -> vec3<f32> {
    return f0 + (1.0 - f0) * pow(1.0 - LdotH, 5.0);
}

// Scale/bias approximation
fn F_AB(perceptual_roughness: f32, NdotV: f32) -> vec2<f32> {
    return textureSampleLevel(brdf_dfg_lut, brdf_dfg_lut_sampler, vec2<f32>(NdotV, perceptual_roughness), 0.0).rg;
}

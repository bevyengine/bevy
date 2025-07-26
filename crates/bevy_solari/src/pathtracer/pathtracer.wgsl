#import bevy_core_pipeline::tonemapping::tonemapping_luminance as luminance
#import bevy_pbr::pbr_functions::calculate_tbn_mikktspace
#import bevy_pbr::utils::{rand_f, rand_vec2f, sample_cosine_hemisphere}
#import bevy_render::maths::PI
#import bevy_render::view::View
#import bevy_solari::brdf::evaluate_brdf
#import bevy_solari::sampling::{sample_random_light, random_light_pdf, sample_ggx_vndf, ggx_vndf_pdf, balance_heuristic, power_heuristic}
#import bevy_solari::scene_bindings::{trace_ray, resolve_ray_hit_full, ResolvedRayHitFull, RAY_T_MIN, RAY_T_MAX}

@group(1) @binding(0) var accumulation_texture: texture_storage_2d<rgba32float, read_write>;
@group(1) @binding(1) var view_output: texture_storage_2d<rgba16float, write>;
@group(1) @binding(2) var<uniform> view: View;

@compute @workgroup_size(8, 8, 1)
fn pathtrace(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if any(global_id.xy >= vec2u(view.viewport.zw)) {
        return;
    }

    let old_color = textureLoad(accumulation_texture, global_id.xy);

    // Setup RNG
    let pixel_index = global_id.x + global_id.y * u32(view.viewport.z);
    let frame_index = u32(old_color.a) * 5782582u;
    var rng = pixel_index + frame_index;

    // Shoot the first ray from the camera
    let pixel_center = vec2<f32>(global_id.xy) + 0.5;
    let jitter = rand_vec2f(&rng) - 0.5;
    let pixel_uv = (pixel_center + jitter) / view.viewport.zw;
    let pixel_ndc = (pixel_uv * 2.0) - 1.0;
    let primary_ray_target = view.world_from_clip * vec4(pixel_ndc.x, -pixel_ndc.y, 1.0, 1.0);
    var ray_origin = view.world_position;
    var ray_direction = normalize((primary_ray_target.xyz / primary_ray_target.w) - ray_origin);
    var ray_t_min = 0.0;

    // Path trace
    var radiance = vec3(0.0);
    var throughput = vec3(1.0);
    var p_bounce = 0.0;
    var bounce_was_perfect_reflection = true;
    var previous_normal = vec3(0.0);
    loop {
        let ray_hit = trace_ray(ray_origin, ray_direction, ray_t_min, RAY_T_MAX, RAY_FLAG_NONE);
        if ray_hit.kind != RAY_QUERY_INTERSECTION_NONE {
            let ray_hit = resolve_ray_hit_full(ray_hit);
            let wo = -ray_direction;

            var mis_weight = 1.0;
            if !bounce_was_perfect_reflection {
                let p_light = random_light_pdf(ray_hit);
                mis_weight = power_heuristic(p_bounce, p_light);
            }
            radiance += mis_weight * throughput * ray_hit.material.emissive;

            // Sample direct lighting, but only if the surface is not mirror-like
            let is_perfectly_specular = ray_hit.material.roughness < 0.0001 && ray_hit.material.metallic > 0.9999;
            if !is_perfectly_specular {
                let direct_lighting = sample_random_light(ray_hit.world_position, ray_hit.world_normal, &rng);
                let pdf_of_bounce = brdf_pdf(wo, direct_lighting.wi, ray_hit);
                mis_weight = power_heuristic(1.0 / direct_lighting.inverse_pdf, pdf_of_bounce);
                let direct_lighting_brdf = evaluate_brdf(ray_hit.world_normal, wo, direct_lighting.wi, ray_hit.material);
                radiance += mis_weight * throughput * direct_lighting.radiance * direct_lighting.inverse_pdf * direct_lighting_brdf;
            }

            // Sample new ray direction from the material BRDF for next bounce
            let next_bounce = importance_sample_next_bounce(wo, ray_hit, &rng);
            ray_direction = next_bounce.wi;
            ray_origin = ray_hit.world_position;
            ray_t_min = RAY_T_MIN;
            p_bounce = next_bounce.pdf;
            bounce_was_perfect_reflection = next_bounce.perfectly_specular_bounce;
            previous_normal = ray_hit.world_normal;

            // Update throughput for next bounce
            let brdf = evaluate_brdf(ray_hit.world_normal, wo, next_bounce.wi, ray_hit.material);
            let cos_theta = dot(next_bounce.wi, ray_hit.world_normal);
            throughput *= (brdf * cos_theta) / next_bounce.pdf;

            // Russian roulette for early termination
            let p = luminance(throughput);
            if rand_f(&rng) > p { break; }
            throughput /= p;
        } else { break; }
    }

    // Camera exposure
    radiance *= view.exposure;

    // Accumulation over time via running average
    let new_color = mix(old_color.rgb, radiance, 1.0 / (old_color.a + 1.0));
    textureStore(accumulation_texture, global_id.xy, vec4(new_color, old_color.a + 1.0));
    textureStore(view_output, global_id.xy, vec4(new_color, 1.0));
}

struct NextBounce {
    wi: vec3<f32>,
    pdf: f32,
    perfectly_specular_bounce: bool,
}

fn importance_sample_next_bounce(wo: vec3<f32>, ray_hit: ResolvedRayHitFull, rng: ptr<function, u32>) -> NextBounce {
    let is_perfectly_specular = ray_hit.material.roughness < 0.0001 && ray_hit.material.metallic > 0.9999;
    if is_perfectly_specular {
        return NextBounce(reflect(-wo, ray_hit.world_normal), 1.0, true);
    }
    let diffuse_weight = mix(mix(0.4f, 0.9f, ray_hit.material.perceptual_roughness), 0.f, ray_hit.material.metallic);
    let specular_weight = 1.0 - diffuse_weight;

    let TBN = calculate_tbn_mikktspace(ray_hit.world_normal, ray_hit.world_tangent);
    let T = TBN[0];
    let B = TBN[1];
    let N = TBN[2];

    let wo_tangent = vec3(dot(wo, T), dot(wo, B), dot(wo, N));

    var wi: vec3<f32>;
    var wi_tangent: vec3<f32>;
    let diffuse_selected = rand_f(rng) < diffuse_weight;
    if diffuse_selected {
        wi = sample_cosine_hemisphere(ray_hit.world_normal, rng);
        wi_tangent = vec3(dot(wi, T), dot(wi, B), dot(wi, N));
    } else {
        wi_tangent = sample_ggx_vndf(wo_tangent, ray_hit.material.roughness, rng);
        wi = wi_tangent.x * T + wi_tangent.y * B + wi_tangent.z * N;
    }

    let diffuse_pdf = dot(wi, ray_hit.world_normal) / PI;
    let specular_pdf = ggx_vndf_pdf(wo_tangent, wi_tangent, ray_hit.material.roughness);
    let pdf = (diffuse_weight * diffuse_pdf) + (specular_weight * specular_pdf);

    return NextBounce(wi, pdf, false);
}

fn brdf_pdf(wo: vec3<f32>, wi: vec3<f32>, ray_hit: ResolvedRayHitFull) -> f32 {
    let diffuse_weight = mix(mix(0.4f, 0.9f, ray_hit.material.roughness), 0.f, ray_hit.material.metallic);
    let specular_weight = 1.0 - diffuse_weight;

    let TBN = calculate_tbn_mikktspace(ray_hit.world_normal, ray_hit.world_tangent);
    let T = TBN[0];
    let B = TBN[1];
    let N = TBN[2];

    let wo_tangent = vec3(dot(wo, T), dot(wo, B), dot(wo, N));
    let wi_tangent = vec3(dot(wi, T), dot(wi, B), dot(wi, N));

    let diffuse_pdf = wi_tangent.z / PI;
    let specular_pdf = ggx_vndf_pdf(wo_tangent, wi_tangent, ray_hit.material.roughness);
    let pdf = (diffuse_weight * diffuse_pdf) + (specular_weight * specular_pdf);
    return pdf;
}

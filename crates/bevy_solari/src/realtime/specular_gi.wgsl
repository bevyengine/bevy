#define_import_path bevy_solari::specular_gi

#import bevy_pbr::pbr_functions::calculate_tbn_mikktspace
#import bevy_render::maths::{orthonormalize, PI}
#import bevy_render::view::View
#import bevy_solari::brdf::{evaluate_brdf, evaluate_specular_brdf}
#import bevy_solari::gbuffer_utils::gpixel_resolve
#import bevy_solari::sampling::{sample_random_light, random_emissive_light_pdf, sample_ggx_vndf, ggx_vndf_pdf, power_heuristic}
#import bevy_solari::scene_bindings::{trace_ray, resolve_ray_hit_full, ResolvedRayHitFull, RAY_T_MIN, RAY_T_MAX}
#import bevy_solari::world_cache::{query_world_cache, get_cell_size, WORLD_CACHE_CELL_LIFETIME}
#import bevy_solari::realtime_bindings::{view_output, gi_reservoirs_a, gbuffer, depth_buffer, view, constants}

const DIFFUSE_GI_REUSE_ROUGHNESS_THRESHOLD: f32 = 0.4;
const SPECULAR_GI_FOR_DI_ROUGHNESS_THRESHOLD: f32 = 0.0225;
const TERMINATE_IN_WORLD_CACHE_THRESHOLD: f32 = 0.03;

@compute @workgroup_size(8, 8, 1)
fn specular_gi(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if any(global_id.xy >= vec2u(view.main_pass_viewport.zw)) { return; }

    let pixel_index = global_id.x + global_id.y * u32(view.main_pass_viewport.z);
    var rng = pixel_index + constants.frame_index;

    let depth = textureLoad(depth_buffer, global_id.xy, 0);
    if depth == 0.0 {
        return;
    }
    let surface = gpixel_resolve(textureLoad(gbuffer, global_id.xy, 0), depth, global_id.xy, view.main_pass_viewport.zw, view.world_from_clip);

    let wo_unnormalized = view.world_position - surface.world_position;
    let wo = normalize(wo_unnormalized);

    var radiance: vec3<f32>;
    var wi: vec3<f32>;
    if surface.material.roughness > DIFFUSE_GI_REUSE_ROUGHNESS_THRESHOLD {
        // Surface is very rough, reuse the ReSTIR GI reservoir
        let gi_reservoir = gi_reservoirs_a[pixel_index];
        wi = normalize(gi_reservoir.sample_point_world_position - surface.world_position);
        radiance = gi_reservoir.radiance * gi_reservoir.unbiased_contribution_weight;
    } else {
        // Surface is glossy or mirror-like, trace a new path
        let TBN = orthonormalize(surface.world_normal);
        let T = TBN[0];
        let B = TBN[1];
        let N = TBN[2];
        let wo_tangent = vec3(dot(wo, T), dot(wo, B), dot(wo, N));
        let wi_tangent = sample_ggx_vndf(wo_tangent, surface.material.roughness, &rng);
        wi = wi_tangent.x * T + wi_tangent.y * B + wi_tangent.z * N;
        let pdf = ggx_vndf_pdf(wo_tangent, wi_tangent, surface.material.roughness);

        // https://d1qx31qr3h6wln.cloudfront.net/publications/mueller21realtime.pdf#subsection.3.4, equation (4)
        let cos_theta = saturate(dot(wo, surface.world_normal));
        var a0 = dot(wo_unnormalized, wo_unnormalized) / (4.0 * PI * cos_theta);
        a0 *= TERMINATE_IN_WORLD_CACHE_THRESHOLD;

        radiance = trace_glossy_path(surface.world_position, wi, surface.material.roughness, pdf, a0, &rng) / pdf;
    }

    let brdf = evaluate_specular_brdf(surface.world_normal, wo, wi, surface.material.base_color, surface.material.metallic,
        surface.material.reflectance, surface.material.perceptual_roughness, surface.material.roughness);
    let cos_theta = saturate(dot(wi, surface.world_normal));
    radiance *= brdf * cos_theta * view.exposure;

    var pixel_color = textureLoad(view_output, global_id.xy);
    pixel_color += vec4(radiance, 0.0);
    textureStore(view_output, global_id.xy, pixel_color);

#ifdef VISUALIZE_WORLD_CACHE
    textureStore(view_output, global_id.xy, vec4(query_world_cache(surface.world_position, surface.world_normal, view.world_position, RAY_T_MAX, WORLD_CACHE_CELL_LIFETIME, &rng) * view.exposure, 1.0));
#endif
}

fn trace_glossy_path(initial_ray_origin: vec3<f32>, initial_wi: vec3<f32>, initial_roughness: f32, initial_p_bounce: f32, a0: f32, rng: ptr<function, u32>) -> vec3<f32> {
    var ray_origin = initial_ray_origin;
    var wi = initial_wi;
    var p_bounce = initial_p_bounce;
    var surface_perfectly_specular = false;
    var path_spread = 0.0;

    // Trace up to three bounces, getting the net throughput from them
    var radiance = vec3(0.0);
    var throughput = vec3(1.0);
    for (var i = 0u; i < 3u; i += 1u) {
        // Trace ray
        let ray = trace_ray(ray_origin, wi, RAY_T_MIN, RAY_T_MAX, RAY_FLAG_NONE);
        if ray.kind == RAY_QUERY_INTERSECTION_NONE { break; }
        let ray_hit = resolve_ray_hit_full(ray);

        let TBN = calculate_tbn_mikktspace(ray_hit.world_normal, ray_hit.world_tangent);
        let T = TBN[0];
        let B = TBN[1];
        let N = TBN[2];

        let wo = -wi;
        let wo_tangent = vec3(dot(wo, T), dot(wo, B), dot(wo, N));

        // Add emissive contribution
        let mis_weight = emissive_mis_weight(i, initial_roughness, p_bounce, ray_hit, surface_perfectly_specular);
        radiance += throughput * mis_weight * ray_hit.material.emissive;

        // Should not perform NEE for mirror-like surfaces
        surface_perfectly_specular = ray_hit.material.roughness <= 0.001 && ray_hit.material.metallic > 0.9999;

        // https://d1qx31qr3h6wln.cloudfront.net/publications/mueller21realtime.pdf#subsection.3.4, equation (3)
        path_spread += sqrt((ray.t * ray.t) / (p_bounce * wo_tangent.z));

        if path_spread * path_spread > a0 * get_cell_size(ray_hit.world_position, view.world_position) {
            // Path spread is wide enough, terminate path in the world cache
            let diffuse_brdf = ray_hit.material.base_color / PI;
            radiance += throughput * diffuse_brdf * query_world_cache(ray_hit.world_position, ray_hit.geometric_world_normal, view.world_position, ray.t, WORLD_CACHE_CELL_LIFETIME, rng);
            break;
        } else if !surface_perfectly_specular {
            // Sample direct lighting (NEE)
            let direct_lighting = sample_random_light(ray_hit.world_position, ray_hit.world_normal, rng);
            let direct_lighting_brdf = evaluate_brdf(ray_hit.world_normal, wo, direct_lighting.wi, ray_hit.material);
            let mis_weight = nee_mis_weight(direct_lighting.inverse_pdf, direct_lighting.brdf_rays_can_hit, wo_tangent, direct_lighting.wi, ray_hit, TBN);
            radiance += throughput * mis_weight * direct_lighting.radiance * direct_lighting.inverse_pdf * direct_lighting_brdf;
        }

        // Sample new ray direction from the GGX BRDF for next bounce
        let wi_tangent = sample_ggx_vndf(wo_tangent, ray_hit.material.roughness, rng);
        wi = wi_tangent.x * T + wi_tangent.y * B + wi_tangent.z * N;
        ray_origin = ray_hit.world_position;

        // Update throughput for next bounce
        p_bounce = ggx_vndf_pdf(wo_tangent, wi_tangent, ray_hit.material.roughness);
        let brdf = evaluate_brdf(N, wo, wi, ray_hit.material);
        let cos_theta = saturate(dot(wi, N));
        throughput *= (brdf * cos_theta) / p_bounce;
    }

    return radiance;
}

fn emissive_mis_weight(i: u32, initial_roughness: f32, p_bounce: f32, ray_hit: ResolvedRayHitFull, previous_surface_perfectly_specular: bool) -> f32 {
    if i != 0u {
        if previous_surface_perfectly_specular { return 1.0; }

        let p_light = random_emissive_light_pdf(ray_hit);
        return power_heuristic(p_bounce, p_light);
    } else {
        // The first bounce gets MIS weight 0.0 or 1.0 depending on if ReSTIR DI shaded using the specular lobe or not
        if initial_roughness <= SPECULAR_GI_FOR_DI_ROUGHNESS_THRESHOLD {
            return 1.0;
        } else {
            return 0.0;
        }
    }
}

fn nee_mis_weight(inverse_p_light: f32, brdf_rays_can_hit: bool, wo_tangent: vec3<f32>, wi: vec3<f32>, ray_hit: ResolvedRayHitFull, TBN: mat3x3<f32>) -> f32 {
    if !brdf_rays_can_hit {
        return 1.0;
    }

    let T = TBN[0];
    let B = TBN[1];
    let N = TBN[2];
    let wi_tangent = vec3(dot(wi, T), dot(wi, B), dot(wi, N));

    let p_light = 1.0 / inverse_p_light;
    let p_bounce = ggx_vndf_pdf(wo_tangent, wi_tangent, ray_hit.material.roughness);
    return power_heuristic(p_light, p_bounce);
}

// Don't adjust the size of this struct without also adjusting GI_RESERVOIR_STRUCT_SIZE.
struct Reservoir {
    sample_point_world_position: vec3<f32>,
    weight_sum: f32,
    radiance: vec3<f32>,
    confidence_weight: f32,
    sample_point_world_normal: vec3<f32>,
    unbiased_contribution_weight: f32,
}

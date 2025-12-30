#define_import_path bevy_solari::sampling

#import bevy_pbr::lighting::D_GGX
#import bevy_pbr::utils::{rand_f, rand_vec2f, rand_u, rand_range_u}
#import bevy_render::maths::{PI_2, orthonormalize}
#import bevy_solari::scene_bindings::{trace_ray, RAY_T_MIN, RAY_T_MAX, light_sources, directional_lights, LightSource, LIGHT_SOURCE_KIND_DIRECTIONAL, resolve_triangle_data_full, resolve_triangle_data_emissive, ResolvedRayHitFull}

fn power_heuristic(f: f32, g: f32) -> f32 {
    return balance_heuristic(f * f, g * g);
}

fn balance_heuristic(f: f32, g: f32) -> f32 {
    let sum = f + g;
    if sum == 0.0 {
        return 0.0;
    }
    return max(0.0, f / sum);
}

// https://gpuopen.com/download/Bounded_VNDF_Sampling_for_Smith-GGX_Reflections.pdf (Listing 1)
fn sample_ggx_vndf(wi_tangent: vec3<f32>, roughness: f32, rng: ptr<function, u32>) -> vec3<f32> {
    if roughness <= 0.001 {
        return vec3(-wi_tangent.xy, wi_tangent.z);
    }

    let i = wi_tangent;
    let rand = rand_vec2f(rng);
    let i_std = normalize(vec3(i.xy * roughness, i.z));
    let phi = PI_2 * rand.x;
    let a = roughness;
    let s = 1.0 + length(vec2(i.xy));
    let a2 = a * a;
    let s2 = s * s;
    let k = (1.0 - a2) * s2 / (s2 + a2 * i.z * i.z);
    let b = select(i_std.z, k * i_std.z, i.z > 0.0);
    let z = fma(1.0 - rand.y, 1.0 + b, -b);
    let sin_theta = sqrt(saturate(1.0 - z * z));
    let o_std = vec3(sin_theta * cos(phi), sin_theta * sin(phi), z);
    let m_std = i_std + o_std;
    let m = normalize(vec3(m_std.xy * roughness, m_std.z));
    return 2.0 * dot(i, m) * m - i;
}

// https://gpuopen.com/download/Bounded_VNDF_Sampling_for_Smith-GGX_Reflections.pdf (Listing 2)
fn ggx_vndf_pdf(wi_tangent: vec3<f32>, wo_tangent: vec3<f32>, roughness: f32) -> f32 {
    let i = wi_tangent;
    let o = wo_tangent;
    let m = normalize(i + o);
    let ndf = D_GGX(roughness, saturate(m.z));
    let ai = roughness * i.xy;
    let len2 = dot(ai, ai);
    let t = sqrt(len2 + i.z * i.z);
    if i.z >= 0.0 {
        let a = roughness;
        let s = 1.0 + length(i.xy);
        let a2 = a * a;
        let s2 = s * s;
        let k = (1.0 - a2) * s2 / (s2 + a2 * i.z * i.z);
        return ndf / (2.0 * (k * i.z + t));
    }
    return ndf * (t - i.z) / (2.0 * len2);
}

struct LightSample {
    light_id: u32,
    seed: u32,
}

struct ResolvedLightSample {
    light_id: u32,
    world_position: vec4<f32>,
    world_normal: vec3<f32>,
    radiance: vec3<f32>,
    inverse_pdf: f32,
}

struct LightContribution {
    light_id: u32,
    world_position: vec4<f32>,
    radiance: vec3<f32>,
    inverse_pdf: f32,
    wi: vec3<f32>,
    brdf_rays_can_hit: bool,
}

fn random_light_contribution(rng: ptr<function, u32>, ray_origin: vec3<f32>, origin_world_normal: vec3<f32>) -> LightContribution {
    let light_id = select_random_light(rng);
    var light_contribution = light_contribution_no_trace(sample_light(rng, light_id), ray_origin, origin_world_normal);
    light_contribution.radiance *= trace_light_visibility(ray_origin, light_contribution.world_position);
    light_contribution.inverse_pdf *= select_random_light_inverse_pdf(light_id);
    return light_contribution;
}

fn hit_random_light_pdf(hit: ResolvedRayHitFull) -> f32 {
    let light_count = arrayLength(&light_sources);
    return 1.0 / (hit.triangle_area * f32(hit.triangle_count) * f32(light_count));
}

fn light_contribution_no_trace(sample: LightSample, ray_origin: vec3<f32>, origin_world_normal: vec3<f32>) -> LightContribution {
    return calculate_light_contribution(resolve_light_sample(sample), ray_origin, origin_world_normal);
}

fn select_random_light(rng: ptr<function, u32>) -> u32 {
    let light_count = arrayLength(&light_sources);
    let light_index = rand_range_u(light_count, rng);
    let light_source = light_sources[light_index];

    var triangle_id = 0u;
    if light_source.kind != LIGHT_SOURCE_KIND_DIRECTIONAL {
        let triangle_count = light_source.kind >> 1u;
        triangle_id = rand_range_u(triangle_count, rng);
    }

    return (light_index << 16u) | triangle_id;
}

fn select_random_light_inverse_pdf(light_id: u32) -> f32 {
    let light_count = arrayLength(&light_sources);
    let light_source = light_sources[light_id >> 16u];
    var triangle_count = 1u;
    if light_source.kind != LIGHT_SOURCE_KIND_DIRECTIONAL {
        triangle_count = light_source.kind >> 1u;
    }
    return f32(light_count) * f32(triangle_count);
}

fn sample_light(rng: ptr<function, u32>, light_id: u32) -> LightSample {
    return LightSample(light_id, rand_u(rng));
}

fn resolve_light_sample(light_sample: LightSample) -> ResolvedLightSample {
    let light_source = light_sources[light_sample.light_id >> 16u];
    if light_source.kind == LIGHT_SOURCE_KIND_DIRECTIONAL {
        let directional_light = directional_lights[light_source.id];

#ifndef NO_DIRECTIONAL_LIGHT_SOFT_SHADOWS
        // Sample a random direction within a cone whose base is the sun approximated as a disk
        // https://www.realtimerendering.com/raytracinggems/unofficial_RayTracingGems_v1.9.pdf#0004286901.INDD%3ASec30%3A305
        var rng = light_sample.seed;
        let random = rand_vec2f(&rng);
        let cos_theta = (1.0 - random.x) + random.x * directional_light.cos_theta_max;
        let sin_theta = sqrt(1.0 - cos_theta * cos_theta);
        let phi = random.y * PI_2;
        let x = cos(phi) * sin_theta;
        let y = sin(phi) * sin_theta;
        var direction_to_light = vec3(x, y, cos_theta);

        // Rotate the ray so that the cone it was sampled from is aligned with the light direction
        direction_to_light = orthonormalize(directional_light.direction_to_light) * direction_to_light;
#else
        let direction_to_light = directional_light.direction_to_light;
#endif

        return ResolvedLightSample(
            light_sample.light_id,
            vec4(direction_to_light, 0.0),
            -direction_to_light,
            directional_light.luminance,
            directional_light.inverse_pdf,
        );
    } else {
        var rng = light_sample.seed;
        let barycentrics = sample_triangle(&rng);
        let triangle_id = light_sample.light_id & 0xFFFFu;
        let triangle_data = resolve_triangle_data_emissive(light_source.id, triangle_id, barycentrics);

        return ResolvedLightSample(
            light_sample.light_id,
            vec4(triangle_data.world_position, 1.0),
            triangle_data.world_normal,
            triangle_data.emissive,
            triangle_data.triangle_area,
        );
    }
}

// https://www.realtimerendering.com/raytracinggems/unofficial_RayTracingGems_v1.9.pdf#0004286901.INDD%3ASec22%3A297
fn sample_triangle(rng: ptr<function, u32>) -> vec3<f32> {
    var barycentrics = rand_vec2f(rng);
    if barycentrics.x + barycentrics.y > 1.0 { barycentrics = 1.0 - barycentrics; }
    return vec3(1.0 - barycentrics.x - barycentrics.y, barycentrics);
}

fn calculate_light_contribution(light_sample: ResolvedLightSample, ray_origin: vec3<f32>, origin_world_normal: vec3<f32>) -> LightContribution {
    let ray = light_sample.world_position.xyz - (light_sample.world_position.w * ray_origin);
    let light_distance = length(ray);
    let wi = ray / light_distance;

    let cos_theta_origin = saturate(dot(wi, origin_world_normal));
    let cos_theta_light = saturate(dot(-wi, light_sample.world_normal));
    let light_distance_squared = light_distance * light_distance;

    let radiance = light_sample.radiance * cos_theta_origin * (cos_theta_light / light_distance_squared);

    return LightContribution(light_sample.light_id, light_sample.world_position, radiance, light_sample.inverse_pdf, wi, light_sample.world_position.w == 1.0);
}

fn trace_light_visibility(ray_origin: vec3<f32>, light_sample_world_position: vec4<f32>) -> f32 {
    var ray_direction = light_sample_world_position.xyz;
    var ray_t_max = RAY_T_MAX;

    if light_sample_world_position.w == 1.0 {
        let ray = ray_direction - ray_origin;
        let dist = length(ray);
        ray_direction = ray / dist;
        ray_t_max = dist - RAY_T_MIN - RAY_T_MIN;
    }

    if ray_t_max < RAY_T_MIN { return 0.0; }

    let ray_hit = trace_ray(ray_origin, ray_direction, RAY_T_MIN, ray_t_max, RAY_FLAG_TERMINATE_ON_FIRST_HIT);
    return f32(ray_hit.kind == RAY_QUERY_INTERSECTION_NONE);
}

fn trace_point_visibility(ray_origin: vec3<f32>, point: vec3<f32>) -> f32 {
    let ray = point - ray_origin;
    let dist = length(ray);
    let ray_direction = ray / dist;

    let ray_t_max = dist - RAY_T_MIN - RAY_T_MIN;
    if ray_t_max < RAY_T_MIN { return 0.0; }

    let ray_hit = trace_ray(ray_origin, ray_direction, RAY_T_MIN, ray_t_max, RAY_FLAG_TERMINATE_ON_FIRST_HIT);
    return f32(ray_hit.kind == RAY_QUERY_INTERSECTION_NONE);
}

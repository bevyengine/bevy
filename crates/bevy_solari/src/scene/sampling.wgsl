#define_import_path bevy_solari::sampling

#import bevy_pbr::utils::{rand_f, rand_vec2f, rand_range_u}
#import bevy_render::maths::{PI, PI_2}
#import bevy_solari::scene_bindings::{trace_ray, RAY_T_MIN, RAY_T_MAX, light_sources, directional_lights, LIGHT_SOURCE_KIND_DIRECTIONAL, resolve_triangle_data_full}

// https://www.realtimerendering.com/raytracinggems/unofficial_RayTracingGems_v1.9.pdf#0004286901.INDD%3ASec28%3A303
fn sample_cosine_hemisphere(normal: vec3<f32>, rng: ptr<function, u32>) -> vec3<f32> {
    let cos_theta = 1.0 - 2.0 * rand_f(rng);
    let phi = PI_2 * rand_f(rng);
    let sin_theta = sqrt(max(1.0 - cos_theta * cos_theta, 0.0));
    let x = normal.x + sin_theta * cos(phi);
    let y = normal.y + sin_theta * sin(phi);
    let z = normal.z + cos_theta;
    return vec3(x, y, z);
}

// https://www.pbr-book.org/3ed-2018/Monte_Carlo_Integration/2D_Sampling_with_Multidimensional_Transformations#UniformlySamplingaHemisphere
fn sample_uniform_hemisphere(normal: vec3<f32>, rng: ptr<function, u32>) -> vec3<f32> {
    let cos_theta = rand_f(rng);
    let phi = PI_2 * rand_f(rng);
    let sin_theta = sqrt(max(1.0 - cos_theta * cos_theta, 0.0));
    let x = sin_theta * cos(phi);
    let y = sin_theta * sin(phi);
    let z = cos_theta;
    return build_orthonormal_basis(normal) * vec3(x, y, z);
}

// https://www.realtimerendering.com/raytracinggems/unofficial_RayTracingGems_v1.9.pdf#0004286901.INDD%3ASec19%3A294
fn sample_disk(disk_radius: f32, rng: ptr<function, u32>) -> vec2<f32> {
    let ab = 2.0 * rand_vec2f(rng) - 1.0;
    let a = ab.x;
    var b = ab.y;
    if (b == 0.0) { b = 1.0; }

    var phi: f32;
    var r: f32;
    if (a * a > b * b) {
        r = disk_radius * a;
        phi = (PI / 4.0) * (b / a);
    } else {
        r = disk_radius * b;
        phi = (PI / 2.0) - (PI / 4.0) * (a / b);
    }

    let x = r * cos(phi);
    let y = r * sin(phi);
    return vec2(x, y);
}

struct SampleRandomLightResult {
    radiance: vec3<f32>,
    inverse_pdf: f32,
}

fn sample_random_light(ray_origin: vec3<f32>, origin_world_normal: vec3<f32>, rng: ptr<function, u32>) -> SampleRandomLightResult {
    let light_sample = generate_random_light_sample(rng);
    let light_contribution = calculate_light_contribution(light_sample, ray_origin, origin_world_normal);
    let visibility = trace_light_visibility(light_sample, ray_origin);
    return SampleRandomLightResult(light_contribution.radiance * visibility, light_contribution.inverse_pdf);
}

struct LightSample {
    light_id: vec2<u32>,
    random: vec2<f32>,
}

struct LightContribution {
    radiance: vec3<f32>,
    inverse_pdf: f32,
}

fn generate_random_light_sample(rng: ptr<function, u32>) -> LightSample {
    let light_count = arrayLength(&light_sources);
    let light_id = rand_range_u(light_count, rng);
    let random = rand_vec2f(rng);

    let light_source = light_sources[light_id];
    var triangle_id = 0u;

    if light_source.kind != LIGHT_SOURCE_KIND_DIRECTIONAL {
        let triangle_count = light_source.kind >> 1u;
        triangle_id = rand_range_u(triangle_count, rng);
    }

    return LightSample(vec2(light_id, triangle_id), random);
}

fn calculate_light_contribution(light_sample: LightSample, ray_origin: vec3<f32>, origin_world_normal: vec3<f32>) -> LightContribution {
    let light_id = light_sample.light_id.x;
    let light_source = light_sources[light_id];

    var light_contribution: LightContribution;
    if light_source.kind == LIGHT_SOURCE_KIND_DIRECTIONAL {
        light_contribution = calculate_directional_light_contribution(light_sample, light_source.id, origin_world_normal);
    } else {
        let triangle_count = light_source.kind >> 1u;
        light_contribution = calculate_emissive_mesh_contribution(light_sample, light_source.id, triangle_count, ray_origin, origin_world_normal);
    }

    let light_count = arrayLength(&light_sources);
    light_contribution.inverse_pdf *= f32(light_count);

    return light_contribution;
}

fn calculate_directional_light_contribution(light_sample: LightSample, directional_light_id: u32, origin_world_normal: vec3<f32>) -> LightContribution {
    let directional_light = directional_lights[directional_light_id];

#ifdef DIRECTIONAL_LIGHT_SOFT_SHADOWS
    // Sample a random direction within a cone whose base is the sun approximated as a disk
    // https://www.realtimerendering.com/raytracinggems/unofficial_RayTracingGems_v1.9.pdf#0004286901.INDD%3ASec30%3A305
    let cos_theta = (1.0 - light_sample.random.x) + light_sample.random.x * directional_light.cos_theta_max;
    let sin_theta = sqrt(1.0 - cos_theta * cos_theta);
    let phi = light_sample.random.y * PI_2;
    let x = cos(phi) * sin_theta;
    let y = sin(phi) * sin_theta;
    var ray_direction = vec3(x, y, cos_theta);

    // Rotate the ray so that the cone it was sampled from is aligned with the light direction
    ray_direction = build_orthonormal_basis(directional_light.direction_to_light) * ray_direction;
#else
    let ray_direction = directional_light.direction_to_light;
#endif

    let cos_theta_origin = saturate(dot(ray_direction, origin_world_normal));
    let radiance = directional_light.luminance * cos_theta_origin;

    return LightContribution(radiance, directional_light.inverse_pdf);
}

fn calculate_emissive_mesh_contribution(light_sample: LightSample, instance_id: u32, triangle_count: u32, ray_origin: vec3<f32>, origin_world_normal: vec3<f32>) -> LightContribution {
    let barycentrics = triangle_barycentrics(light_sample.random);
    let triangle_id = light_sample.light_id.y;

    let triangle_data = resolve_triangle_data_full(instance_id, triangle_id, barycentrics);

    let light_distance = distance(ray_origin, triangle_data.world_position);
    let ray_direction = (triangle_data.world_position - ray_origin) / light_distance;
    let cos_theta_origin = saturate(dot(ray_direction, origin_world_normal));
    let cos_theta_light = saturate(dot(-ray_direction, triangle_data.world_normal));
    let light_distance_squared = light_distance * light_distance;

    let radiance = triangle_data.material.emissive.rgb * cos_theta_origin * (cos_theta_light / light_distance_squared);
    let inverse_pdf = f32(triangle_count) * triangle_data.triangle_area;

    return LightContribution(radiance, inverse_pdf);
}

fn trace_light_visibility(light_sample: LightSample, ray_origin: vec3<f32>) -> f32 {
    let light_id = light_sample.light_id.x;
    let light_source = light_sources[light_id];

    if light_source.kind == LIGHT_SOURCE_KIND_DIRECTIONAL {
        return trace_directional_light_visibility(light_sample, light_source.id, ray_origin);
    } else {
        return trace_emissive_mesh_visibility(light_sample, light_source.id, ray_origin);
    }
}

fn trace_directional_light_visibility(light_sample: LightSample, directional_light_id: u32, ray_origin: vec3<f32>) -> f32 {
    let directional_light = directional_lights[directional_light_id];

#ifdef DIRECTIONAL_LIGHT_SOFT_SHADOWS
    // Sample a random direction within a cone whose base is the sun approximated as a disk
    // https://www.realtimerendering.com/raytracinggems/unofficial_RayTracingGems_v1.9.pdf#0004286901.INDD%3ASec30%3A305
    let cos_theta = (1.0 - light_sample.random.x) + light_sample.random.x * directional_light.cos_theta_max;
    let sin_theta = sqrt(1.0 - cos_theta * cos_theta);
    let phi = light_sample.random.y * PI_2;
    let x = cos(phi) * sin_theta;
    let y = sin(phi) * sin_theta;
    var ray_direction = vec3(x, y, cos_theta);

    // Rotate the ray so that the cone it was sampled from is aligned with the light direction
    ray_direction = build_orthonormal_basis(directional_light.direction_to_light) * ray_direction;
#else
    let ray_direction = directional_light.direction_to_light;
#endif

    let ray_hit = trace_ray(ray_origin, ray_direction, RAY_T_MIN, RAY_T_MAX, RAY_FLAG_TERMINATE_ON_FIRST_HIT);
    return f32(ray_hit.kind == RAY_QUERY_INTERSECTION_NONE);
}

fn trace_emissive_mesh_visibility(light_sample: LightSample, instance_id: u32, ray_origin: vec3<f32>) -> f32 {
    let barycentrics = triangle_barycentrics(light_sample.random);
    let triangle_id = light_sample.light_id.y;

    let triangle_data = resolve_triangle_data_full(instance_id, triangle_id, barycentrics);

    return trace_point_visibility(ray_origin, triangle_data.world_position);
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

// https://www.realtimerendering.com/raytracinggems/unofficial_RayTracingGems_v1.9.pdf#0004286901.INDD%3ASec22%3A297
fn triangle_barycentrics(random: vec2<f32>) -> vec3<f32> {
    var barycentrics = random;
    if barycentrics.x + barycentrics.y > 1.0 { barycentrics = 1.0 - barycentrics; }
    return vec3(1.0 - barycentrics.x - barycentrics.y, barycentrics);
}

// https://jcgt.org/published/0006/01/01/paper.pdf
fn build_orthonormal_basis(normal: vec3<f32>) -> mat3x3<f32> {
    let sign = select(-1.0, 1.0, normal.z >= 0.0);
    let a = -1.0 / (sign + normal.z);
    let b = normal.x * normal.y * a;
    let tangent = vec3(1.0 + sign * normal.x * normal.x * a, sign * b, -sign * normal.x);
    let bitangent = vec3(b, sign + normal.y * normal.y * a, -normal.y);
    return mat3x3(tangent, bitangent, normal);
}

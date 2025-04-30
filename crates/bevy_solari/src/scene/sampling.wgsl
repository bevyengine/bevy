#define_import_path bevy_solari::sampling

#import bevy_pbr::utils::{rand_f, rand_vec2f, rand_range_u}
#import bevy_render::maths::PI
#import bevy_solari::scene_bindings::{trace_ray, RAY_T_MIN, RAY_T_MAX, light_sources, directional_lights, LIGHT_SOURCE_KIND_DIRECTIONAL, resolve_triangle_data_full}

// https://www.realtimerendering.com/raytracinggems/unofficial_RayTracingGems_v1.9.pdf#0004286901.INDD%3ASec28%3A303
fn sample_cosine_hemisphere(normal: vec3<f32>, rng: ptr<function, u32>) -> vec3<f32> {
    let cos_theta = 1.0 - 2.0 * rand_f(rng);
    let phi = 2.0 * PI * rand_f(rng);
    let sin_theta = sqrt(max(1.0 - cos_theta * cos_theta, 0.0));
    let x = normal.x + sin_theta * cos(phi);
    let y = normal.y + sin_theta * sin(phi);
    let z = normal.z + cos_theta;
    return vec3(x, y, z);
}

fn sample_random_light(ray_origin: vec3<f32>, origin_world_normal: vec3<f32>, rng: ptr<function, u32>) -> vec3<f32> {
    let light_count = arrayLength(&light_sources);
    let light_id = rand_range_u(light_count, rng);
    let light = light_sources[light_id];

    var radiance: vec3<f32>;
    if light.kind == LIGHT_SOURCE_KIND_DIRECTIONAL {
        radiance = sample_directional_light(ray_origin, origin_world_normal, light.id, rng);
    } else {
        radiance = sample_emissive_mesh(ray_origin, origin_world_normal, light.id, light.kind >> 1u, rng);
    }

    let inverse_pdf = f32(light_count);

    return radiance * inverse_pdf;
}

fn sample_directional_light(ray_origin: vec3<f32>, origin_world_normal: vec3<f32>, directional_light_id: u32, rng: ptr<function, u32>) -> vec3<f32> {
    let light = directional_lights[directional_light_id];

    // Angular diameter of the sun projected onto a disk as viewed from earth = ~0.5 degrees
    // https://en.wikipedia.org/wiki/Angular_diameter#Use_in_astronomy
    // cos(0.25)
    let cos_theta_max = 0.99999048072;

    // Sample a random direction within a cone whose base is the sun approximated as a disk with radius ~= 0.25 degrees
    // https://www.realtimerendering.com/raytracinggems/unofficial_RayTracingGems_v1.9.pdf#0004286901.INDD%3ASec30%3A305
    let r = rand_vec2f(rng);
    let cos_theta = (1.0 - r.x) + r.x * cos_theta_max;
    let sin_theta = sqrt(1.0 - cos_theta * cos_theta);
    let phi = r.y * 2.0 * PI;
    let x = cos(phi) * sin_theta;
    let y = sin(phi) * sin_theta;
    var ray_direction = vec3(x, y, cos_theta);

    // Rotate the ray so that the cone it was sampled from is aligned with the light direction
    ray_direction = build_orthonormal_basis(light.direction_to_light) * ray_direction;

    let ray_hit = trace_ray(ray_origin, ray_direction, RAY_T_MIN, RAY_T_MAX, RAY_FLAG_TERMINATE_ON_FIRST_HIT);
    let visibility = f32(ray_hit.kind == RAY_QUERY_INTERSECTION_NONE);

    let cos_theta_origin = saturate(dot(ray_direction, origin_world_normal));

    // No need to divide by the pdf, because we also need to divide by the solid angle to convert from illuminance to luminance, and they cancel out
    return light.illuminance.rgb * visibility * cos_theta_origin;
}

fn sample_emissive_mesh(ray_origin: vec3<f32>, origin_world_normal: vec3<f32>, instance_id: u32, triangle_count: u32, rng: ptr<function, u32>) -> vec3<f32> {
    let barycentrics = sample_triangle_barycentrics(rng);
    let triangle_id = rand_range_u(triangle_count, rng);

    let triangle_data = resolve_triangle_data_full(instance_id, triangle_id, barycentrics);

    let light_distance = distance(ray_origin, triangle_data.world_position);
    let ray_direction = (triangle_data.world_position - ray_origin) / light_distance;
    let cos_theta_origin = saturate(dot(ray_direction, origin_world_normal));
    let cos_theta_light = saturate(dot(-ray_direction, triangle_data.world_normal));
    let light_distance_squared = light_distance * light_distance;

    let ray_hit = trace_ray(ray_origin, ray_direction, RAY_T_MIN, light_distance - RAY_T_MIN - RAY_T_MIN, RAY_FLAG_TERMINATE_ON_FIRST_HIT);
    let visibility = f32(ray_hit.kind == RAY_QUERY_INTERSECTION_NONE);

    let radiance = triangle_data.material.emissive.rgb * visibility * cos_theta_origin * (cos_theta_light / light_distance_squared);

    let inverse_pdf = f32(triangle_count) * triangle_data.triangle_area;

    return radiance * inverse_pdf;
}

// https://www.realtimerendering.com/raytracinggems/unofficial_RayTracingGems_v1.9.pdf#0004286901.INDD%3ASec22%3A297
fn sample_triangle_barycentrics(rng: ptr<function, u32>) -> vec3<f32> {
    var barycentrics = rand_vec2f(rng);
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

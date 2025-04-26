#define_import_path bevy_solari::sampling

#import bevy_pbr::utils::{rand_f, rand_vec2f, rand_range_u}
#import bevy_render::maths::PI
#import bevy_solari::scene_bindings::{trace_ray, RAY_T_MIN, light_sources, directional_lights, LIGHT_SOURCE_KIND_DIRECTIONAL, resolve_triangle_data_full}

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

    var radiance = vec3(0.0);
    if light.kind == LIGHT_SOURCE_KIND_DIRECTIONAL {
        // TODO
    } else {
        radiance = sample_emissive_mesh(ray_origin, origin_world_normal, light.id, light.kind >> 1u, rng);
    }

    return radiance / f32(light_count);
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

    return radiance / inverse_pdf;
}

// https://www.realtimerendering.com/raytracinggems/unofficial_RayTracingGems_v1.9.pdf#0004286901.INDD%3ASec22%3A297
fn sample_triangle_barycentrics(rng: ptr<function, u32>) -> vec3<f32> {
    var barycentrics = rand_vec2f(rng);
    if barycentrics.x + barycentrics.y > 1.0 { barycentrics = 1.0 - barycentrics; }
    return vec3(1.0 - barycentrics.x - barycentrics.y, barycentrics);
}

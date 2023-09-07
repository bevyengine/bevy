#define_import_path bevy_solari::utils

#import bevy_solari::scene_types SolariVertex, unpack_vertex, TEXTURE_MAP_NONE
#import bevy_solari::scene_bindings tlas, uniforms, emissive_object_triangle_counts, emissive_object_indices, mesh_material_indices, transforms, index_buffers, vertex_buffers, materials, sample_texture_map
#import bevy_solari::global_illumination::view_bindings view
#import bevy_pbr::utils PI

fn rand_u(state: ptr<function, u32>) -> u32 {
    *state = *state * 747796405u + 2891336453u;
    let word = ((*state >> ((*state >> 28u) + 4u)) ^ *state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn rand_f(state: ptr<function, u32>) -> f32 {
    *state = *state * 747796405u + 2891336453u;
    let word = ((*state >> ((*state >> 28u) + 4u)) ^ *state) * 277803737u;
    return f32((word >> 22u) ^ word) * bitcast<f32>(0x2f800004u);
}

fn rand_vec2f(state: ptr<function, u32>) -> vec2<f32> {
    return vec2(rand_f(state), rand_f(state));
}

fn rand_range_u(n: u32, state: ptr<function, u32>) -> u32 {
    return rand_u(state) % n;
}

fn sample_cosine_hemisphere(normal: vec3<f32>, state: ptr<function, u32>) -> vec3<f32> {
    let cos_theta = 2.0 * rand_f(state) - 1.0;
    let phi = 2.0 * PI * rand_f(state);
    let sin_theta = sqrt(max(1.0 - cos_theta * cos_theta, 0.0));
    let sin_phi = sin(phi);
    let cos_phi = cos(phi);
    let unit_sphere_direction = normalize(vec3(sin_theta * cos_phi, cos_theta, sin_theta * sin_phi));
    return normal + unit_sphere_direction;
}

fn trace_ray(ray_origin: vec3<f32>, ray_direction: vec3<f32>, ray_t_min: f32) -> RayIntersection {
    let ray_flags = RAY_FLAG_NONE;
    let ray_cull_mask = 0xFFu;
    let ray_t_max = 10000.0;
    let ray = RayDesc(ray_flags, ray_cull_mask, ray_t_min, ray_t_max, ray_origin, ray_direction);

    var rq: ray_query;
    rayQueryInitialize(&rq, tlas, ray);
    rayQueryProceed(&rq);
    return rayQueryGetCommittedIntersection(&rq);
}

fn trace_light_visibility(ray_origin: vec3<f32>, light_position: vec3<f32>, light_distance: f32) -> f32 {
    let ray_flags = RAY_FLAG_TERMINATE_ON_FIRST_HIT;
    let ray_cull_mask = 0xFFu;
    let ray_t_min = 0.01;
    let ray_t_max = light_distance - 0.01;
    let ray_direction = (light_position - ray_origin) / light_distance;
    let ray = RayDesc(ray_flags, ray_cull_mask, ray_t_min, ray_t_max, ray_origin, ray_direction);
    var rq: ray_query;
    rayQueryInitialize(&rq, tlas, ray);
    rayQueryProceed(&rq);
    let ray_hit = rayQueryGetCommittedIntersection(&rq);
    return f32(ray_hit.kind == RAY_QUERY_INTERSECTION_NONE);
}

fn generate_tbn(normal: vec3<f32>) -> mat3x3<f32> {
    var bitangent = vec3(0.0, 1.0, 0.0);

    let n_dot_up = dot(normal, bitangent);
    if 1.0 - abs(n_dot_up) <= 0.0000001 {
        if n_dot_up > 0.0 {
            bitangent = vec3(0.0, 0.0, 1.0);
        } else {
            bitangent = vec3(0.0, 0.0, -1.0);
        }
    }

    let tangent = normalize(cross(bitangent, normal));
    bitangent = cross(normal, tangent);

    return mat3x3(tangent, bitangent, normal);
}

fn sample_sunlight(ray_origin: vec3<f32>, origin_world_normal: vec3<f32>, state: ptr<function, u32>) -> vec3<f32> {
    if all(uniforms.sun_color == 0.0) {
        return vec3(0.0);
    }

    let r = rand_vec2f(state);
    let cos_theta = (1.0 - r.x) + r.x * 0.99998918271;
    let sin_theta = sqrt(1.0 - cos_theta * cos_theta);
    let phi = r.y * 2.0 * PI;
    var ray_direction = vec3(vec2(cos(phi), sin(phi)) * sin_theta, cos_theta);

    ray_direction = generate_tbn(uniforms.sun_direction) * ray_direction;

    let ray = RayDesc(RAY_FLAG_TERMINATE_ON_FIRST_HIT, 0xFFu, 0.0, 10000.0, ray_origin + (0.001 * origin_world_normal), ray_direction);
    var rq: ray_query;
    rayQueryInitialize(&rq, tlas, ray);
    rayQueryProceed(&rq);
    let ray_hit = rayQueryGetCommittedIntersection(&rq);
    let sun_visible = f32(ray_hit.kind == RAY_QUERY_INTERSECTION_NONE);

    return uniforms.sun_color * sun_visible * 0.00006796703;
}

struct RandomLightSample {
    world_position: vec3<f32>,
    light_distance: f32,
    light: vec3<f32>,
    inverse_pdf: f32,
}

fn sample_unshadowed_direct_lighting(ray_origin: vec3<f32>, origin_world_normal: vec3<f32>, light_count: u32, state: ptr<function, u32>) -> RandomLightSample {
    let light_i = rand_range_u(light_count, state);
    let triangle_count = emissive_object_triangle_counts[light_i];
    let triangle_i = rand_range_u(triangle_count, state);

    let light_object_i = emissive_object_indices[light_i];
    let light_mm_indices = mesh_material_indices[light_object_i];
    let light_transform = transforms[light_object_i];
    let mesh_index = light_mm_indices >> 16u;
    let material_index = light_mm_indices & 0xFFFFu;
    let index_buffer = &index_buffers[mesh_index].buffer;
    let vertex_buffer = &vertex_buffers[mesh_index].buffer;
    var material = materials[material_index];
    let indices_i = (triangle_i * 3u) + vec3(0u, 1u, 2u);
    let indices = vec3((*index_buffer)[indices_i.x], (*index_buffer)[indices_i.y], (*index_buffer)[indices_i.z]);
    let vertices = array<SolariVertex, 3>(unpack_vertex((*vertex_buffer)[indices.x]), unpack_vertex((*vertex_buffer)[indices.y]), unpack_vertex((*vertex_buffer)[indices.z]));

    var r = rand_vec2f(state);
    if r.x + r.y > 1.0 { r = 1.0 - r; }
    let barycentrics = vec3(r, 1.0 - r.x - r.y);

    let uv = mat3x2(vertices[0].uv, vertices[1].uv, vertices[2].uv) * barycentrics;

    let local_position = mat3x3(vertices[0].local_position, vertices[1].local_position, vertices[2].local_position) * barycentrics;
    let world_position = (light_transform * vec4(local_position, 1.0)).xyz;

    let light_distance = distance(ray_origin, world_position);
    let ray_direction = (world_position - ray_origin) / light_distance;

    let local_normal = mat3x3(vertices[0].local_normal, vertices[1].local_normal, vertices[2].local_normal) * barycentrics;
    var world_normal = normalize(mat3x3(light_transform[0].xyz, light_transform[1].xyz, light_transform[2].xyz) * local_normal);
    if material.normal_map_texture_index != TEXTURE_MAP_NONE {
        let local_tangent = mat3x3(vertices[0].local_tangent.xyz, vertices[1].local_tangent.xyz, vertices[2].local_tangent.xyz) * barycentrics;
        let world_tangent = normalize(mat3x3(light_transform[0].xyz, light_transform[1].xyz, light_transform[2].xyz) * local_tangent);
        let N = world_normal;
        let T = world_tangent;
        let B = vertices[0].local_tangent.w * cross(N, T);
        let Nt = sample_texture_map(material.normal_map_texture_index, uv).rgb;
        world_normal = normalize(Nt.x * T + Nt.y * B + Nt.z * N);
    }

    if material.emissive_texture_index != TEXTURE_MAP_NONE {
        material.emissive *= sample_texture_map(material.emissive_texture_index, uv).rgb;
    }

    let cos_theta_origin = saturate(dot(ray_direction, origin_world_normal));
    let cos_theta_light = saturate(dot(-ray_direction, world_normal));
    let light_distance_squared = light_distance * light_distance;
    let light = material.emissive * cos_theta_origin * (cos_theta_light / light_distance_squared);

    let triangle_edge0 = vertices[0].local_position - vertices[1].local_position;
    let triangle_edge1 = vertices[0].local_position - vertices[2].local_position;
    let triangle_area = length(cross(triangle_edge0, triangle_edge1)) / 2.0;

    let inverse_pdf = f32(light_count * triangle_count) * triangle_area;

    return RandomLightSample(world_position, light_distance, light, inverse_pdf);
}

fn sample_direct_lighting(ray_origin: vec3<f32>, origin_world_normal: vec3<f32>, state: ptr<function, u32>) -> vec3<f32> {
    let light_count = arrayLength(&emissive_object_indices);
    let unshadowed_light = sample_unshadowed_direct_lighting(ray_origin, origin_world_normal, light_count, state);
    let visibility = trace_light_visibility(ray_origin, unshadowed_light.world_position, unshadowed_light.light_distance);
    let emissive_light = unshadowed_light.light * unshadowed_light.inverse_pdf * visibility;

    let sunlight = sample_sunlight(ray_origin, origin_world_normal, state);

    return emissive_light + sunlight;
}

fn depth_to_world_position(depth: f32, uv: vec2<f32>) -> vec3<f32> {
    let clip_xy = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - 2.0 * uv.y);
    let t = view.inverse_projection * vec4<f32>(clip_xy, depth, 1.0);
    let view_xyz = t.xyz / t.w;
    return view_xyz - view.world_position;
}

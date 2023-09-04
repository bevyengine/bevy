struct SolariUniforms {
    frame_count: u32,
    sun_direction: vec3<f32>,
    sun_color: vec3<f32>,
}

struct SolariIndexBuffer {
    buffer: array<u32>,
}

struct SolariVertexBuffer {
    buffer: array<SolariPackedVertex>,
}

// The size of a vertex is 48 bytes of data
//
// The size of the SolariVertex struct when used in an
// array is padded to 64 bytes due to WGSL alignment rules
//
// This struct is properly 48 bytes
struct SolariPackedVertex {
    b0: vec4<f32>,
    b1: vec4<f32>,
    local_tangent: vec4<f32>,
}

fn unpack_vertex(packed: SolariPackedVertex) -> SolariVertex {
    var vertex: SolariVertex;
    vertex.local_position = packed.b0.xyz;
    vertex.local_normal = vec3(packed.b0.w, packed.b1.xy);
    vertex.local_tangent = packed.local_tangent;
    vertex.uv = packed.b1.zw;
    return vertex;
}

struct SolariVertex {
    local_position: vec3<f32>,
    local_normal: vec3<f32>,
    local_tangent: vec4<f32>,
    uv: vec2<f32>,
}

const TEXTURE_MAP_NONE = 0xffffffffu;

struct SolariMaterial {
    base_color: vec4<f32>,
    base_color_map_index: u32,
    normal_map_index: u32,
    emissive: vec3<f32>,
    emissive_map_index: u32,
}

struct SolariSampledMaterial {
    base_color: vec3<f32>,
    emissive: vec3<f32>,
}

fn sample_texture_map(i: u32, uv: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(texture_maps[i], texture_map_samplers[i], uv, 0.0);
}

fn sample_material(material: SolariMaterial, uv: vec2<f32>) -> SolariSampledMaterial {
    var m: SolariSampledMaterial;

    m.base_color = material.base_color.rgb;
    if material.base_color_map_index != TEXTURE_MAP_NONE {
        m.base_color *= sample_texture_map(material.base_color_map_index, uv).rgb;
    }

    m.emissive = material.emissive;
    if material.emissive_map_index != TEXTURE_MAP_NONE {
        m.emissive *= sample_texture_map(material.emissive_map_index, uv).rgb;
    }

    return m;
}

struct SolariRayHit {
    world_position: vec3<f32>,
    world_normal: vec3<f32>,
    geometric_world_normal: vec3<f32>,
    uv: vec2<f32>,
    material: SolariSampledMaterial,
}

fn map_ray_hit(ray_hit: RayIntersection) -> SolariRayHit {
    let mm_indices = mesh_material_indices[ray_hit.instance_custom_index];
    let mesh_index = mm_indices >> 16u;
    let material_index = mm_indices & 0xFFFFu;

    let index_buffer = &index_buffers[mesh_index].buffer;
    let vertex_buffer = &vertex_buffers[mesh_index].buffer;
    let material = materials[material_index];

    let indices_i = (ray_hit.primitive_index * 3u) + vec3(0u, 1u, 2u);
    let indices = vec3((*index_buffer)[indices_i.x], (*index_buffer)[indices_i.y], (*index_buffer)[indices_i.z]);
    let vertices = array<SolariVertex, 3>(unpack_vertex((*vertex_buffer)[indices.x]), unpack_vertex((*vertex_buffer)[indices.y]), unpack_vertex((*vertex_buffer)[indices.z]));
    let barycentrics = vec3(1.0 - ray_hit.barycentrics.x - ray_hit.barycentrics.y, ray_hit.barycentrics);

    let local_position = mat3x3(vertices[0].local_position, vertices[1].local_position, vertices[2].local_position) * barycentrics;
    let world_position = (ray_hit.object_to_world * vec4(local_position, 1.0)).xyz;

    let uv = mat3x2(vertices[0].uv, vertices[1].uv, vertices[2].uv) * barycentrics;

    let transform = transforms[ray_hit.instance_custom_index];
    let local_normal = mat3x3(vertices[0].local_normal, vertices[1].local_normal, vertices[2].local_normal) * barycentrics;
    var world_normal = normalize(mat3x3(transform[0].xyz, transform[1].xyz, transform[2].xyz) * local_normal);
    let geometric_world_normal = world_normal;
    if material.normal_map_index != TEXTURE_MAP_NONE {
        let local_tangent = mat3x3(vertices[0].local_tangent.xyz, vertices[1].local_tangent.xyz, vertices[2].local_tangent.xyz) * barycentrics;
        let world_tangent = normalize(mat3x3(transform[0].xyz, transform[1].xyz, transform[2].xyz) * local_tangent);
        let N = world_normal;
        let T = world_tangent;
        let B = vertices[0].local_tangent.w * cross(N, T);
        let Nt = sample_texture_map(material.normal_map_index, uv).rgb;
        world_normal = normalize(Nt.x * T + Nt.y * B + Nt.z * N);
    }

    let sampled_material = sample_material(material, uv);

    return SolariRayHit(world_position, world_normal, geometric_world_normal, uv, sampled_material);
}

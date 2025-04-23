#define_import_path bevy_solari::scene_bindings

struct InstanceGeometryIds {
    vertex_buffer_id: u32,
    vertex_buffer_offset: u32,
    index_buffer_id: u32,
    index_buffer_offset: u32,
}

struct VertexBuffer { vertices: array<PackedVertex> }

struct IndexBuffer { indices: array<u32> }

struct PackedVertex {
    a: vec4<f32>,
    b: vec4<f32>,
    tangent: vec4<f32>,
}

struct Vertex {
    position: vec3<f32>,
    normal: vec3<f32>,
    uv: vec2<f32>,
    tangent: vec4<f32>,
}

fn unpack_vertex(packed: PackedVertex) -> Vertex {
    var vertex: Vertex;
    vertex.position = packed.a.xyz;
    vertex.normal = vec3(packed.a.w, packed.b.xy);
    vertex.uv = packed.b.zw;
    vertex.tangent = packed.tangent;
    return vertex;
}

struct Material {
    base_color: vec4<f32>,
    emissive: vec4<f32>,
    base_color_texture_id: u32,
    normal_map_texture_id: u32,
    emissive_texture_id: u32,
    _padding: u32,
}

const TEXTURE_MAP_NONE = 0xFFFFFFFFu;

struct LightSource {
    kind: u32, // 1 bit for kind, 31 bits for extra data
    id: u32,
}

const LIGHT_SOURCE_KIND_EMISSIVE_MESH = 0u;
const LIGHT_SOURCE_KIND_DIRECTIONAL = 1u;

struct DirectionalLight {
    direction_to_light: vec3<f32>,
    _padding: u32,
    color: vec4<f32>,
}

@group(0) @binding(0) var<storage> vertex_buffers: binding_array<VertexBuffer>;
@group(0) @binding(1) var<storage> index_buffers: binding_array<IndexBuffer>;
@group(0) @binding(2) var textures: binding_array<texture_2d<f32>>;
@group(0) @binding(3) var samplers: binding_array<sampler>;
@group(0) @binding(4) var<storage> materials: array<Material>;
@group(0) @binding(5) var tlas: acceleration_structure;
@group(0) @binding(6) var<storage> transforms: array<mat4x4<f32>>;
@group(0) @binding(7) var<storage> geometry_ids: array<InstanceGeometryIds>;
@group(0) @binding(8) var<storage> material_ids: array<u32>; // TODO: Store material_id in instance_custom_index instead?
@group(0) @binding(9) var<storage> light_sources: array<LightSource>;
@group(0) @binding(10) var<storage> directional_lights: array<DirectionalLight>;

const RAY_NO_CULL = 0xFFu;

fn trace_ray(ray_origin: vec3<f32>, ray_direction: vec3<f32>, ray_t_min: f32, ray_t_max: f32) -> RayIntersection {
    let ray = RayDesc(RAY_FLAG_NONE, RAY_NO_CULL, ray_t_min, ray_t_max, ray_origin, ray_direction);
    var rq: ray_query;
    rayQueryInitialize(&rq, tlas, ray);
    rayQueryProceed(&rq);
    return rayQueryGetCommittedIntersection(&rq);
}

fn sample_texture(id: u32, uv: vec2<f32>) -> vec3<f32> {
    return textureSampleLevel(textures[id], samplers[id], uv, 0.0).rgb; // TODO: Mipmap
}

struct ResolvedMaterial {
    base_color: vec3<f32>,
    emissive: vec3<f32>,
}

struct ResolvedRayHitFull {
    world_position: vec3<f32>,
    world_normal: vec3<f32>,
    geometric_world_normal: vec3<f32>,
    uv: vec2<f32>,
    material: ResolvedMaterial,
}

fn resolve_material(material: Material, uv: vec2<f32>) -> ResolvedMaterial {
    var m: ResolvedMaterial;

    m.base_color = material.base_color.rgb;
    if material.base_color_texture_id != TEXTURE_MAP_NONE {
        m.base_color *= sample_texture(material.base_color_texture_id, uv);
    }

    m.emissive = material.emissive.rgb;
    if material.emissive_texture_id != TEXTURE_MAP_NONE {
        m.emissive *= sample_texture(material.emissive_texture_id, uv);
    }

    return m;
}

fn resolve_ray_hit_full(ray_hit: RayIntersection) -> ResolvedRayHitFull {
    let instance_geometry_ids = geometry_ids[ray_hit.instance_id];
    let material_id = material_ids[ray_hit.instance_id];

    let index_buffer = &index_buffers[instance_geometry_ids.index_buffer_id].indices;
    let vertex_buffer = &vertex_buffers[instance_geometry_ids.vertex_buffer_id].vertices;
    let material = materials[material_id];

    let indices_i = (ray_hit.primitive_index * 3u) + vec3(0u, 1u, 2u) + instance_geometry_ids.index_buffer_offset;
    let indices = vec3((*index_buffer)[indices_i.x], (*index_buffer)[indices_i.y], (*index_buffer)[indices_i.z]) + instance_geometry_ids.vertex_buffer_offset;
    let vertices = array<Vertex, 3>(unpack_vertex((*vertex_buffer)[indices.x]), unpack_vertex((*vertex_buffer)[indices.y]), unpack_vertex((*vertex_buffer)[indices.z]));
    let barycentrics = vec3(1.0 - ray_hit.barycentrics.x - ray_hit.barycentrics.y, ray_hit.barycentrics);

    let transform = transforms[ray_hit.instance_id];
    let local_position = mat3x3(vertices[0].position, vertices[1].position, vertices[2].position) * barycentrics;
    let world_position = (transform * vec4(local_position, 1.0)).xyz;

    let uv = mat3x2(vertices[0].uv, vertices[1].uv, vertices[2].uv) * barycentrics;

    let local_normal = mat3x3(vertices[0].normal, vertices[1].normal, vertices[2].normal) * barycentrics; // TODO: Use barycentric lerp, ray_hit.object_to_world, cross product geo normal
    var world_normal = normalize(mat3x3(transform[0].xyz, transform[1].xyz, transform[2].xyz) * local_normal);
    let geometric_world_normal = world_normal;
    if material.normal_map_texture_id != TEXTURE_MAP_NONE {
        let local_tangent = mat3x3(vertices[0].tangent.xyz, vertices[1].tangent.xyz, vertices[2].tangent.xyz) * barycentrics;
        let world_tangent = normalize(mat3x3(transform[0].xyz, transform[1].xyz, transform[2].xyz) * local_tangent);
        let N = world_normal;
        let T = world_tangent;
        let B = vertices[0].tangent.w * cross(N, T);
        let Nt = sample_texture(material.normal_map_texture_id, uv);
        world_normal = normalize(Nt.x * T + Nt.y * B + Nt.z * N);
    }

    let resolved_material = resolve_material(material, uv);

    return ResolvedRayHitFull(world_position, world_normal, geometric_world_normal, uv, resolved_material);
}

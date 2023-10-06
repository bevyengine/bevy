#define_import_path bevy_solari::scene_bindings

#import bevy_solari::scene_types SolariIndexBuffer, SolariVertexBuffer, SolariUniforms, SolariMaterial, SolariSampledMaterial, SolariRayHit, SolariVertex, unpack_vertex, TEXTURE_MAP_NONE

@group(0) @binding(0) var tlas: acceleration_structure;
@group(0) @binding(1) var<storage> mesh_material_indices: array<u32>;
@group(0) @binding(2) var<storage> index_buffers: binding_array<SolariIndexBuffer>;
@group(0) @binding(3) var<storage> vertex_buffers: binding_array<SolariVertexBuffer>;
@group(0) @binding(4) var<storage> transforms: array<mat4x4<f32>>;
@group(0) @binding(5) var<storage> materials: array<SolariMaterial>;
@group(0) @binding(6) var texture_maps: binding_array<texture_2d<f32>>;
@group(0) @binding(7) var texture_map_samplers: binding_array<sampler>;
@group(0) @binding(8) var<storage> emissive_object_indices: array<u32>;
@group(0) @binding(9) var<storage> emissive_object_triangle_counts: array<u32>;
@group(0) @binding(10) var<uniform> uniforms: SolariUniforms;

fn sample_texture_map(i: u32, uv: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(texture_maps[i], texture_map_samplers[i], uv, 0.0);
}

fn sample_material(material: SolariMaterial, uv: vec2<f32>) -> SolariSampledMaterial {
    var m: SolariSampledMaterial;

    m.base_color = material.base_color.rgb;
    if material.base_color_texture_index != TEXTURE_MAP_NONE {
        m.base_color *= sample_texture_map(material.base_color_texture_index, uv).rgb;
    }

    m.emissive = material.emissive;
    if material.emissive_texture_index != TEXTURE_MAP_NONE {
        m.emissive *= sample_texture_map(material.emissive_texture_index, uv).rgb;
    }

    return m;
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
    if material.normal_map_texture_index != TEXTURE_MAP_NONE {
        let local_tangent = mat3x3(vertices[0].local_tangent.xyz, vertices[1].local_tangent.xyz, vertices[2].local_tangent.xyz) * barycentrics;
        let world_tangent = normalize(mat3x3(transform[0].xyz, transform[1].xyz, transform[2].xyz) * local_tangent);
        let N = world_normal;
        let T = world_tangent;
        let B = vertices[0].local_tangent.w * cross(N, T);
        let Nt = sample_texture_map(material.normal_map_texture_index, uv).rgb;
        world_normal = normalize(Nt.x * T + Nt.y * B + Nt.z * N);
    }

    let sampled_material = sample_material(material, uv);

    return SolariRayHit(world_position, world_normal, geometric_world_normal, uv, sampled_material);
}

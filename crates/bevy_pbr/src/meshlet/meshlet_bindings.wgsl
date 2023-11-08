#define_import_path bevy_pbr::meshlet_bindings

#import bevy_pbr::mesh_types::Mesh
#import bevy_render::view::View

struct PackedMeshletVertex {
    a: vec4<f32>,
    b: vec4<f32>,
    tangent: vec4<f32>,
}

struct MeshletVertex {
    position: vec3<f32>,
    normal: vec3<f32>,
    uv: vec2<f32>,
    tangent: vec4<f32>,
}

fn unpack_meshlet_vertex(packed: PackedMeshletVertex) -> MeshletVertex {
    var vertex: MeshletVertex;
    vertex.position = packed.a.xyz;
    vertex.normal = vec3(packed.a.w, packed.b.xy);
    vertex.uv = packed.b.zw;
    vertex.tangent = packed.tangent;
    return vertex;
}

struct Meshlet {
    start_vertex_id: u32,
    start_index_id: u32,
    vertex_count: u32,
    triangle_count: u32,
}

struct MeshletBoundingSphere {
    center: vec3<f32>,
    radius: f32,
}

struct DrawIndexedIndirect {
    vertex_count: atomic<u32>,
    instance_count: u32,
    base_index: u32,
    vertex_offset: u32,
    base_instance: u32,
}

struct PartialDerivatives {
    barycentrics: vec3<f32>,
    ddx: vec3<f32>,
    ddy: vec3<f32>,
}

// https://github.com/JuanDiegoMontoya/Frogfood/blob/main/data/shaders/visbuffer/VisbufferResolve.frag.glsl#L43-L79
fn compute_derivatives(vertex_clip_positions: array<vec4<f32>, 3>, ndc_uv: vec2<f32>, screen_size: vec2<f32>) -> PartialDerivatives {
    var result: PartialDerivatives;

    let inv_w = 1.0 / vec3(vertex_clip_positions[0].w, vertex_clip_positions[1].w, vertex_clip_positions[2].w);
    let ndc_0 = vertex_clip_positions[0].xy * inv_w[0];
    let ndc_1 = vertex_clip_positions[1].xy * inv_w[1];
    let ndc_2 = vertex_clip_positions[2].xy * inv_w[2];

    let inv_det = 1.0 / determinant(mat2x2(ndc_2 - ndc_1, ndc_0 - ndc_1));
    result.ddx = vec3(ndc_1.y - ndc_2.y, ndc_2.y - ndc_0.y, ndc_0.y - ndc_1.y) * inv_det * inv_w;
    result.ddy = vec3(ndc_2.x - ndc_1.x, ndc_0.x - ndc_2.x, ndc_1.x - ndc_0.x) * inv_det * inv_w;

    var ddx_sum = dot(result.ddx, vec3(1.0));
    var ddy_sum = dot(result.ddy, vec3(1.0));

    let delta_v = ndc_uv - ndc_0;
    let interp_inv_w = inv_w.x + delta_v.x * ddx_sum + delta_v.y * ddy_sum;
    let interp_w = 1.0 / interp_inv_w;

    result.barycentrics = vec3(
        interp_w * (delta_v.x * result.ddx.x + delta_v.y * result.ddy.x + inv_w.x),
        interp_w * (delta_v.x * result.ddx.y + delta_v.y * result.ddy.y),
        interp_w * (delta_v.x * result.ddx.z + delta_v.y * result.ddy.z),
    );

    result.ddx *= 2.0 / screen_size.x;
    result.ddy *= 2.0 / screen_size.y;
    ddx_sum *= 2.0 / screen_size.x;
    ddy_sum *= 2.0 / screen_size.y;

    let interp_ddx_w = 1.0 / (interp_inv_w + ddx_sum);
    let interp_ddy_w = 1.0 / (interp_inv_w + ddy_sum);

    result.ddx = interp_ddx_w * (result.barycentrics * interp_inv_w + result.ddx) - result.barycentrics;
    result.ddy = interp_ddy_w * (result.barycentrics * interp_inv_w + result.ddy) - result.barycentrics;
    return result;
}

@group(#{MESHLET_BIND_GROUP}) @binding(0) var<storage, read> meshlets: array<Meshlet>;
@group(#{MESHLET_BIND_GROUP}) @binding(1) var<storage, read> meshlet_instance_uniforms: array<Mesh>;
@group(#{MESHLET_BIND_GROUP}) @binding(2) var<storage, read> meshlet_thread_instance_ids: array<u32>;
@group(#{MESHLET_BIND_GROUP}) @binding(3) var<storage, read> meshlet_thread_meshlet_ids: array<u32>;

#ifdef MESHLET_CULLING_PASS
@group(0) @binding(4) var<storage, read> meshlet_bounding_spheres: array<MeshletBoundingSphere>;
@group(0) @binding(5) var<storage, read_write> draw_command_buffer: DrawIndexedIndirect;
@group(0) @binding(6) var<storage, write> draw_index_buffer: array<u32>;
@group(0) @binding(7) var<uniform> view: View;
#endif

#ifdef MESHLET_VISIBILITY_BUFFER_PASS
@group(0) @binding(4) var<storage, read> meshlet_vertex_data: array<PackedMeshletVertex>;
@group(0) @binding(5) var<storage, read> meshlet_vertex_ids: array<u32>;
@group(0) @binding(6) var<storage, read> meshlet_indices: array<u32>; // packed u8's
@group(0) @binding(7) var<storage, read> meshlet_instance_material_ids: array<u32>;
@group(0) @binding(8) var<uniform> view: View;

fn get_meshlet_index(index_id: u32) -> u32 {
    let packed_index = meshlet_indices[index_id / 4u];
    let bit_offset = (index_id % 4u) * 8u;
    return extractBits(packed_index, bit_offset, 8u);
}
#endif

#ifdef MESHLET_MESH_MATERIAL_PASS
@group(1) @binding(4) var<storage, read> meshlet_vertex_data: array<PackedMeshletVertex>;
@group(1) @binding(5) var<storage, read> meshlet_vertex_ids: array<u32>;
@group(1) @binding(6) var<storage, read> meshlet_indices: array<u32>; // packed u8's
@group(1) @binding(7) var meshlet_visibility_buffer: texture_2d<u32>;

fn get_meshlet_index(index_id: u32) -> u32 {
    let packed_index = meshlet_indices[index_id / 4u];
    let bit_offset = (index_id % 4u) * 8u;
    return extractBits(packed_index, bit_offset, 8u);
}
#endif

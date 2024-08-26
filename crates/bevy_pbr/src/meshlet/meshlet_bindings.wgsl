#define_import_path bevy_pbr::meshlet_bindings

#import bevy_pbr::mesh_types::Mesh
#import bevy_render::view::View
#import bevy_pbr::prepass_bindings::PreviousViewUniforms

struct PackedMeshletVertex {
    a: vec4<f32>,
    b: vec4<f32>,
    tangent: vec4<f32>,
}

// TODO: Octahedral encode normal, remove tangent and derive from UV derivatives
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

struct MeshletBoundingSpheres {
    self_culling: MeshletBoundingSphere,
    self_lod: MeshletBoundingSphere,
    parent_lod: MeshletBoundingSphere,
}

struct MeshletBoundingSphere {
    center: vec3<f32>,
    radius: f32,
}

struct DispatchIndirectArgs {
    x: atomic<u32>,
    y: u32,
    z: u32,
}

struct DrawIndirectArgs {
    vertex_count: u32,
    instance_count: atomic<u32>,
    first_vertex: u32,
    first_instance: u32,
}

#ifdef MESHLET_FILL_CLUSTER_BUFFERS_PASS
var<push_constant> cluster_count: u32;
@group(0) @binding(0) var<storage, read> meshlet_instance_meshlet_counts_prefix_sum: array<u32>; // Per entity instance
@group(0) @binding(1) var<storage, read> meshlet_instance_meshlet_slice_starts: array<u32>; // Per entity instance
@group(0) @binding(2) var<storage, read_write> meshlet_cluster_instance_ids: array<u32>; // Per cluster
@group(0) @binding(3) var<storage, read_write> meshlet_cluster_meshlet_ids: array<u32>; // Per cluster
#endif

#ifdef MESHLET_CULLING_PASS
var<push_constant> meshlet_raster_cluster_rightmost_slot: u32;
@group(0) @binding(0) var<storage, read> meshlet_cluster_meshlet_ids: array<u32>; // Per cluster
@group(0) @binding(1) var<storage, read> meshlet_bounding_spheres: array<MeshletBoundingSpheres>; // Per meshlet
@group(0) @binding(2) var<storage, read> meshlet_cluster_instance_ids: array<u32>; // Per cluster
@group(0) @binding(3) var<storage, read> meshlet_instance_uniforms: array<Mesh>; // Per entity instance
@group(0) @binding(4) var<storage, read> meshlet_view_instance_visibility: array<u32>; // 1 bit per entity instance, packed as a bitmask
@group(0) @binding(5) var<storage, read_write> meshlet_second_pass_candidates: array<atomic<u32>>; // 1 bit per cluster , packed as a bitmask
@group(0) @binding(6) var<storage, read_write> meshlet_software_raster_indirect_args: DispatchIndirectArgs; // Single object shared between all workgroups/clusters/triangles
@group(0) @binding(7) var<storage, read_write> meshlet_hardware_raster_indirect_args: DrawIndirectArgs; // Single object shared between all workgroups/clusters/triangles
@group(0) @binding(8) var<storage, read_write> meshlet_raster_clusters: array<u32>; // Single object shared between all workgroups/clusters/triangles
@group(0) @binding(9) var depth_pyramid: texture_2d<f32>; // From the end of the last frame for the first culling pass, and from the first raster pass for the second culling pass
@group(0) @binding(10) var<uniform> view: View;
@group(0) @binding(11) var<uniform> previous_view: PreviousViewUniforms;

fn should_cull_instance(instance_id: u32) -> bool {
    let bit_offset = instance_id % 32u;
    let packed_visibility = meshlet_view_instance_visibility[instance_id / 32u];
    return bool(extractBits(packed_visibility, bit_offset, 1u));
}

// TODO: Load 4x per workgroup instead of once per thread?
fn cluster_is_second_pass_candidate(cluster_id: u32) -> bool {
    let packed_candidates = meshlet_second_pass_candidates[cluster_id / 32u];
    let bit_offset = cluster_id % 32u;
    return bool(extractBits(packed_candidates, bit_offset, 1u));
}
#endif

#ifdef MESHLET_VISIBILITY_BUFFER_RASTER_PASS
@group(0) @binding(0) var<storage, read> meshlet_cluster_meshlet_ids: array<u32>; // Per cluster
@group(0) @binding(1) var<storage, read> meshlets: array<Meshlet>; // Per meshlet
@group(0) @binding(2) var<storage, read> meshlet_indices: array<u32>; // Many per meshlet
@group(0) @binding(3) var<storage, read> meshlet_vertex_ids: array<u32>; // Many per meshlet
@group(0) @binding(4) var<storage, read> meshlet_vertex_data: array<PackedMeshletVertex>; // Many per meshlet
@group(0) @binding(5) var<storage, read> meshlet_cluster_instance_ids: array<u32>; // Per cluster
@group(0) @binding(6) var<storage, read> meshlet_instance_uniforms: array<Mesh>; // Per entity instance
@group(0) @binding(7) var<storage, read> meshlet_raster_clusters: array<u32>; // Single object shared between all workgroups/clusters/triangles
@group(0) @binding(8) var<storage, read> meshlet_software_raster_cluster_count: u32;
#ifdef MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT
@group(0) @binding(9) var<storage, read_write> meshlet_visibility_buffer: array<atomic<u64>>; // Per pixel
#else
@group(0) @binding(9) var<storage, read_write> meshlet_visibility_buffer: array<atomic<u32>>; // Per pixel
#endif
@group(0) @binding(10) var<uniform> view: View;

// TODO: Load only twice, instead of 3x in cases where you load 3 indices per thread?
fn get_meshlet_index(index_id: u32) -> u32 {
    let packed_index = meshlet_indices[index_id / 4u];
    let bit_offset = (index_id % 4u) * 8u;
    return extractBits(packed_index, bit_offset, 8u);
}
#endif

#ifdef MESHLET_MESH_MATERIAL_PASS
@group(1) @binding(0) var<storage, read> meshlet_visibility_buffer: array<u64>; // Per pixel
@group(1) @binding(1) var<storage, read> meshlet_cluster_meshlet_ids: array<u32>; // Per cluster
@group(1) @binding(2) var<storage, read> meshlets: array<Meshlet>; // Per meshlet
@group(1) @binding(3) var<storage, read> meshlet_indices: array<u32>; // Many per meshlet
@group(1) @binding(4) var<storage, read> meshlet_vertex_ids: array<u32>; // Many per meshlet
@group(1) @binding(5) var<storage, read> meshlet_vertex_data: array<PackedMeshletVertex>; // Many per meshlet
@group(1) @binding(6) var<storage, read> meshlet_cluster_instance_ids: array<u32>; // Per cluster
@group(1) @binding(7) var<storage, read> meshlet_instance_uniforms: array<Mesh>; // Per entity instance

// TODO: Load only twice, instead of 3x in cases where you load 3 indices per thread?
fn get_meshlet_index(index_id: u32) -> u32 {
    let packed_index = meshlet_indices[index_id / 4u];
    let bit_offset = (index_id % 4u) * 8u;
    return extractBits(packed_index, bit_offset, 8u);
}
#endif

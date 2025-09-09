#define_import_path bevy_pbr::meshlet_bindings

#import bevy_pbr::mesh_types::Mesh
#import bevy_render::view::View
#import bevy_pbr::prepass_bindings::PreviousViewUniforms
#import bevy_pbr::utils::octahedral_decode_signed

struct BvhNode {
    aabbs: array<MeshletAabbErrorOffset, 8>,
    lod_bounds: array<vec4<f32>, 8>,
    child_counts: array<u32, 2>,
    _padding: vec2<u32>,
}

struct Meshlet {
    start_vertex_position_bit: u32,
    start_vertex_attribute_id: u32,
    start_index_id: u32,
    packed_a: u32,
    packed_b: u32,
    min_vertex_position_channel_x: f32,
    min_vertex_position_channel_y: f32,
    min_vertex_position_channel_z: f32,
}

fn get_meshlet_vertex_count(meshlet: ptr<function, Meshlet>) -> u32 {
    return extractBits((*meshlet).packed_a, 0u, 8u);
}

fn get_meshlet_triangle_count(meshlet: ptr<function, Meshlet>) -> u32 {
    return extractBits((*meshlet).packed_a, 8u, 8u);
}

struct MeshletCullData {
    aabb: MeshletAabbErrorOffset,
    lod_group_sphere: vec4<f32>,
}

struct MeshletAabb {
    center: vec3<f32>,
    half_extent: vec3<f32>,
}

struct MeshletAabbErrorOffset {
    center_and_error: vec4<f32>,
    half_extent_and_child_offset: vec4<f32>,
}

fn get_aabb(aabb: ptr<function, MeshletAabbErrorOffset>) -> MeshletAabb {
    return MeshletAabb(
        (*aabb).center_and_error.xyz,
        (*aabb).half_extent_and_child_offset.xyz,
    );
}

fn get_aabb_error(aabb: ptr<function, MeshletAabbErrorOffset>) -> f32 {
    return (*aabb).center_and_error.w;
}

fn get_aabb_child_offset(aabb: ptr<function, MeshletAabbErrorOffset>) -> u32 {
    return bitcast<u32>((*aabb).half_extent_and_child_offset.w);
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

// Either a BVH node or a meshlet, along with the instance it is associated with.
// Refers to BVH nodes in `meshlet_bvh_cull_queue` and `meshlet_second_pass_bvh_queue`, where `offset` is the index into `meshlet_bvh_nodes`.
// Refers to meshlets in `meshlet_meshlet_cull_queue` and `meshlet_raster_clusters`.
// In `meshlet_meshlet_cull_queue`, `offset` is the index into `meshlet_cull_data`.
// In `meshlet_raster_clusters`, `offset` is the index into `meshlets`.
struct InstancedOffset {
    instance_id: u32,
    offset: u32,
}

const CENTIMETERS_PER_METER = 100.0;

#ifdef MESHLET_INSTANCE_CULLING_PASS
struct Constants { scene_instance_count: u32 }
var<push_constant> constants: Constants;

// Cull data
@group(0) @binding(0) var depth_pyramid: texture_2d<f32>;
@group(0) @binding(1) var<uniform> view: View;
@group(0) @binding(2) var<uniform> previous_view: PreviousViewUniforms;

// Per entity instance data
@group(0) @binding(3) var<storage, read> meshlet_instance_uniforms: array<Mesh>;
@group(0) @binding(4) var<storage, read> meshlet_view_instance_visibility: array<u32>; // 1 bit per entity instance, packed as a bitmask
@group(0) @binding(5) var<storage, read> meshlet_instance_aabbs: array<MeshletAabb>;
@group(0) @binding(6) var<storage, read> meshlet_instance_bvh_root_nodes: array<u32>;

// BVH cull queue data
@group(0) @binding(7) var<storage, read_write> meshlet_bvh_cull_count_write: atomic<u32>;
@group(0) @binding(8) var<storage, read_write> meshlet_bvh_cull_dispatch: DispatchIndirectArgs;
@group(0) @binding(9) var<storage, read_write> meshlet_bvh_cull_queue: array<InstancedOffset>;

// Second pass queue data
#ifdef MESHLET_FIRST_CULLING_PASS
@group(0) @binding(10) var<storage, read_write> meshlet_second_pass_instance_count: atomic<u32>;
@group(0) @binding(11) var<storage, read_write> meshlet_second_pass_instance_dispatch: DispatchIndirectArgs;
@group(0) @binding(12) var<storage, read_write> meshlet_second_pass_instance_candidates: array<u32>;
#else
@group(0) @binding(10) var<storage, read> meshlet_second_pass_instance_count: u32;
@group(0) @binding(11) var<storage, read> meshlet_second_pass_instance_candidates: array<u32>;
#endif
#endif

#ifdef MESHLET_BVH_CULLING_PASS
struct Constants { read_from_front: u32, rightmost_slot: u32 }
var<push_constant> constants: Constants;

// Cull data
@group(0) @binding(0) var depth_pyramid: texture_2d<f32>; // From the end of the last frame for the first culling pass, and from the first raster pass for the second culling pass
@group(0) @binding(1) var<uniform> view: View;
@group(0) @binding(2) var<uniform> previous_view: PreviousViewUniforms;

// Global mesh data
@group(0) @binding(3) var<storage, read> meshlet_bvh_nodes: array<BvhNode>;

// Per entity instance data
@group(0) @binding(4) var<storage, read> meshlet_instance_uniforms: array<Mesh>;

// BVH cull queue data
@group(0) @binding(5) var<storage, read> meshlet_bvh_cull_count_read: u32;
@group(0) @binding(6) var<storage, read_write> meshlet_bvh_cull_count_write: atomic<u32>;
@group(0) @binding(7) var<storage, read_write> meshlet_bvh_cull_dispatch: DispatchIndirectArgs;
@group(0) @binding(8) var<storage, read_write> meshlet_bvh_cull_queue: array<InstancedOffset>;

// Meshlet cull queue data
@group(0) @binding(9) var<storage, read_write> meshlet_meshlet_cull_count_early: atomic<u32>;
@group(0) @binding(10) var<storage, read_write> meshlet_meshlet_cull_count_late: atomic<u32>;
@group(0) @binding(11) var<storage, read_write> meshlet_meshlet_cull_dispatch_early: DispatchIndirectArgs;
@group(0) @binding(12) var<storage, read_write> meshlet_meshlet_cull_dispatch_late: DispatchIndirectArgs;
@group(0) @binding(13) var<storage, read_write> meshlet_meshlet_cull_queue: array<InstancedOffset>;

// Second pass queue data
#ifdef MESHLET_FIRST_CULLING_PASS
@group(0) @binding(14) var<storage, read_write> meshlet_second_pass_bvh_count: atomic<u32>;
@group(0) @binding(15) var<storage, read_write> meshlet_second_pass_bvh_dispatch: DispatchIndirectArgs;
@group(0) @binding(16) var<storage, read_write> meshlet_second_pass_bvh_queue: array<InstancedOffset>;
#endif
#endif

#ifdef MESHLET_CLUSTER_CULLING_PASS
struct Constants { rightmost_slot: u32 }
var<push_constant> constants: Constants;

// Cull data
@group(0) @binding(0) var depth_pyramid: texture_2d<f32>; // From the end of the last frame for the first culling pass, and from the first raster pass for the second culling pass
@group(0) @binding(1) var<uniform> view: View;
@group(0) @binding(2) var<uniform> previous_view: PreviousViewUniforms;

// Global mesh data
@group(0) @binding(3) var<storage, read> meshlet_cull_data: array<MeshletCullData>;

// Per entity instance data
@group(0) @binding(4) var<storage, read> meshlet_instance_uniforms: array<Mesh>;

// Raster queue data
@group(0) @binding(5) var<storage, read_write> meshlet_software_raster_indirect_args: DispatchIndirectArgs;
@group(0) @binding(6) var<storage, read_write> meshlet_hardware_raster_indirect_args: DrawIndirectArgs;
@group(0) @binding(7) var<storage, read> meshlet_previous_raster_counts: array<u32>;
@group(0) @binding(8) var<storage, read_write> meshlet_raster_clusters: array<InstancedOffset>;

// Meshlet cull queue data
@group(0) @binding(9) var<storage, read> meshlet_meshlet_cull_count_read: u32;

// Second pass queue data
#ifdef MESHLET_FIRST_CULLING_PASS
@group(0) @binding(10) var<storage, read_write> meshlet_meshlet_cull_count_write: atomic<u32>;
@group(0) @binding(11) var<storage, read_write> meshlet_meshlet_cull_dispatch: DispatchIndirectArgs;
@group(0) @binding(12) var<storage, read_write> meshlet_meshlet_cull_queue: array<InstancedOffset>;
#else
@group(0) @binding(10) var<storage, read> meshlet_meshlet_cull_queue: array<InstancedOffset>;
#endif
#endif

#ifdef MESHLET_VISIBILITY_BUFFER_RASTER_PASS
@group(0) @binding(0) var<storage, read> meshlet_raster_clusters: array<InstancedOffset>; // Per cluster
@group(0) @binding(1) var<storage, read> meshlets: array<Meshlet>; // Per meshlet
@group(0) @binding(2) var<storage, read> meshlet_indices: array<u32>; // Many per meshlet
@group(0) @binding(3) var<storage, read> meshlet_vertex_positions: array<u32>; // Many per meshlet
@group(0) @binding(4) var<storage, read> meshlet_instance_uniforms: array<Mesh>; // Per entity instance
@group(0) @binding(5) var<storage, read> meshlet_previous_raster_counts: array<u32>;
@group(0) @binding(6) var<storage, read> meshlet_software_raster_cluster_count: u32;
#ifdef MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT
@group(0) @binding(7) var meshlet_visibility_buffer: texture_storage_2d<r64uint, atomic>;
#else
@group(0) @binding(7) var meshlet_visibility_buffer: texture_storage_2d<r32uint, atomic>;
#endif
@group(0) @binding(8) var<uniform> view: View;

// TODO: Load only twice, instead of 3x in cases where you load 3 indices per thread?
fn get_meshlet_vertex_id(index_id: u32) -> u32 {
    let packed_index = meshlet_indices[index_id / 4u];
    let bit_offset = (index_id % 4u) * 8u;
    return extractBits(packed_index, bit_offset, 8u);
}

fn get_meshlet_vertex_position(meshlet: ptr<function, Meshlet>, vertex_id: u32) -> vec3<f32> {
    // Get bitstream start for the vertex
    let unpacked = unpack4xU8((*meshlet).packed_b);
    let bits_per_channel = unpacked.xyz;
    let bits_per_vertex = bits_per_channel.x + bits_per_channel.y + bits_per_channel.z;
    var start_bit = (*meshlet).start_vertex_position_bit + (vertex_id * bits_per_vertex);

    // Read each vertex channel from the bitstream
    var vertex_position_packed = vec3(0u);
    for (var i = 0u; i < 3u; i++) {
        let lower_word_index = start_bit / 32u;
        let lower_word_bit_offset = start_bit & 31u;
        var next_32_bits = meshlet_vertex_positions[lower_word_index] >> lower_word_bit_offset;
        if lower_word_bit_offset + bits_per_channel[i] > 32u {
            next_32_bits |= meshlet_vertex_positions[lower_word_index + 1u] << (32u - lower_word_bit_offset);
        }
        vertex_position_packed[i] = extractBits(next_32_bits, 0u, bits_per_channel[i]);
        start_bit += bits_per_channel[i];
    }

    // Remap [0, range_max - range_min] vec3<u32> to [range_min, range_max] vec3<f32>
    var vertex_position = vec3<f32>(vertex_position_packed) + vec3(
        (*meshlet).min_vertex_position_channel_x,
        (*meshlet).min_vertex_position_channel_y,
        (*meshlet).min_vertex_position_channel_z,
    );

    // Reverse vertex quantization
    let vertex_position_quantization_factor = unpacked.w;
    vertex_position /= f32(1u << vertex_position_quantization_factor) * CENTIMETERS_PER_METER;

    return vertex_position;
}
#endif

#ifdef MESHLET_MESH_MATERIAL_PASS
@group(2) @binding(0) var meshlet_visibility_buffer: texture_storage_2d<r64uint, read>;
@group(2) @binding(1) var<storage, read> meshlet_raster_clusters: array<InstancedOffset>; // Per cluster
@group(2) @binding(2) var<storage, read> meshlets: array<Meshlet>; // Per meshlet
@group(2) @binding(3) var<storage, read> meshlet_indices: array<u32>; // Many per meshlet
@group(2) @binding(4) var<storage, read> meshlet_vertex_positions: array<u32>; // Many per meshlet
@group(2) @binding(5) var<storage, read> meshlet_vertex_normals: array<u32>; // Many per meshlet
@group(2) @binding(6) var<storage, read> meshlet_vertex_uvs: array<vec2<f32>>; // Many per meshlet
@group(2) @binding(7) var<storage, read> meshlet_instance_uniforms: array<Mesh>; // Per entity instance

// TODO: Load only twice, instead of 3x in cases where you load 3 indices per thread?
fn get_meshlet_vertex_id(index_id: u32) -> u32 {
    let packed_index = meshlet_indices[index_id / 4u];
    let bit_offset = (index_id % 4u) * 8u;
    return extractBits(packed_index, bit_offset, 8u);
}

fn get_meshlet_vertex_position(meshlet: ptr<function, Meshlet>, vertex_id: u32) -> vec3<f32> {
    // Get bitstream start for the vertex
    let unpacked = unpack4xU8((*meshlet).packed_b);
    let bits_per_channel = unpacked.xyz;
    let bits_per_vertex = bits_per_channel.x + bits_per_channel.y + bits_per_channel.z;
    var start_bit = (*meshlet).start_vertex_position_bit + (vertex_id * bits_per_vertex);

    // Read each vertex channel from the bitstream
    var vertex_position_packed = vec3(0u);
    for (var i = 0u; i < 3u; i++) {
        let lower_word_index = start_bit / 32u;
        let lower_word_bit_offset = start_bit & 31u;
        var next_32_bits = meshlet_vertex_positions[lower_word_index] >> lower_word_bit_offset;
        if lower_word_bit_offset + bits_per_channel[i] > 32u {
            next_32_bits |= meshlet_vertex_positions[lower_word_index + 1u] << (32u - lower_word_bit_offset);
        }
        vertex_position_packed[i] = extractBits(next_32_bits, 0u, bits_per_channel[i]);
        start_bit += bits_per_channel[i];
    }

    // Remap [0, range_max - range_min] vec3<u32> to [range_min, range_max] vec3<f32>
    var vertex_position = vec3<f32>(vertex_position_packed) + vec3(
        (*meshlet).min_vertex_position_channel_x,
        (*meshlet).min_vertex_position_channel_y,
        (*meshlet).min_vertex_position_channel_z,
    );

    // Reverse vertex quantization
    let vertex_position_quantization_factor = unpacked.w;
    vertex_position /= f32(1u << vertex_position_quantization_factor) * CENTIMETERS_PER_METER;

    return vertex_position;
}

fn get_meshlet_vertex_normal(meshlet: ptr<function, Meshlet>, vertex_id: u32) -> vec3<f32> {
    let packed_normal = meshlet_vertex_normals[(*meshlet).start_vertex_attribute_id + vertex_id];
    return octahedral_decode_signed(unpack2x16snorm(packed_normal));
}

fn get_meshlet_vertex_uv(meshlet: ptr<function, Meshlet>, vertex_id: u32) -> vec2<f32> {
    return meshlet_vertex_uvs[(*meshlet).start_vertex_attribute_id + vertex_id];
}
#endif

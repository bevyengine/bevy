#define_import_path bevy_pbr::meshlet_bindings

#import bevy_pbr::mesh_types::Mesh
#import bevy_render::view::View
#import bevy_pbr::prepass_bindings::PreviousViewUniforms
#import bevy_pbr::utils::octahedral_decode_signed

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

struct MeshletBoundingSpheres {
    culling_sphere: MeshletBoundingSphere,
    lod_group_sphere: MeshletBoundingSphere,
    lod_parent_group_sphere: MeshletBoundingSphere,
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

const CENTIMETERS_PER_METER = 100.0;

#ifdef MESHLET_FILL_CLUSTER_BUFFERS_PASS
var<push_constant> scene_instance_count: u32;
@group(0) @binding(0) var<storage, read> meshlet_instance_meshlet_counts: array<u32>; // Per entity instance
@group(0) @binding(1) var<storage, read> meshlet_instance_meshlet_slice_starts: array<u32>; // Per entity instance
@group(0) @binding(2) var<storage, read_write> meshlet_cluster_instance_ids: array<u32>; // Per cluster
@group(0) @binding(3) var<storage, read_write> meshlet_cluster_meshlet_ids: array<u32>; // Per cluster
@group(0) @binding(4) var<storage, read_write> meshlet_global_cluster_count: atomic<u32>; // Single object shared between all workgroups
#endif

#ifdef MESHLET_CULLING_PASS
struct Constants { scene_cluster_count: u32, meshlet_raster_cluster_rightmost_slot: u32 }
var<push_constant> constants: Constants;
@group(0) @binding(0) var<storage, read> meshlet_cluster_meshlet_ids: array<u32>; // Per cluster
@group(0) @binding(1) var<storage, read> meshlet_bounding_spheres: array<MeshletBoundingSpheres>; // Per meshlet
@group(0) @binding(2) var<storage, read> meshlet_simplification_errors: array<u32>; // Per meshlet
@group(0) @binding(3) var<storage, read> meshlet_cluster_instance_ids: array<u32>; // Per cluster
@group(0) @binding(4) var<storage, read> meshlet_instance_uniforms: array<Mesh>; // Per entity instance
@group(0) @binding(5) var<storage, read> meshlet_view_instance_visibility: array<u32>; // 1 bit per entity instance, packed as a bitmask
@group(0) @binding(6) var<storage, read_write> meshlet_second_pass_candidates: array<atomic<u32>>; // 1 bit per cluster , packed as a bitmask
@group(0) @binding(7) var<storage, read_write> meshlet_software_raster_indirect_args: DispatchIndirectArgs; // Single object shared between all workgroups
@group(0) @binding(8) var<storage, read_write> meshlet_hardware_raster_indirect_args: DrawIndirectArgs; // Single object shared between all workgroups
@group(0) @binding(9) var<storage, read_write> meshlet_raster_clusters: array<u32>; // Single object shared between all workgroups
@group(0) @binding(10) var depth_pyramid: texture_2d<f32>; // From the end of the last frame for the first culling pass, and from the first raster pass for the second culling pass
@group(0) @binding(11) var<uniform> view: View;
@group(0) @binding(12) var<uniform> previous_view: PreviousViewUniforms;

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
@group(0) @binding(3) var<storage, read> meshlet_vertex_positions: array<u32>; // Many per meshlet
@group(0) @binding(4) var<storage, read> meshlet_cluster_instance_ids: array<u32>; // Per cluster
@group(0) @binding(5) var<storage, read> meshlet_instance_uniforms: array<Mesh>; // Per entity instance
@group(0) @binding(6) var<storage, read> meshlet_raster_clusters: array<u32>; // Single object shared between all workgroups
@group(0) @binding(7) var<storage, read> meshlet_software_raster_cluster_count: u32;
#ifdef MESHLET_VISIBILITY_BUFFER_RASTER_PASS_OUTPUT
@group(0) @binding(8) var meshlet_visibility_buffer: texture_storage_2d<r64uint, atomic>;
#else
@group(0) @binding(8) var meshlet_visibility_buffer: texture_storage_2d<r32uint, atomic>;
#endif
@group(0) @binding(9) var<uniform> view: View;

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
@group(1) @binding(0) var meshlet_visibility_buffer: texture_storage_2d<r64uint, read>;
@group(1) @binding(1) var<storage, read> meshlet_cluster_meshlet_ids: array<u32>; // Per cluster
@group(1) @binding(2) var<storage, read> meshlets: array<Meshlet>; // Per meshlet
@group(1) @binding(3) var<storage, read> meshlet_indices: array<u32>; // Many per meshlet
@group(1) @binding(4) var<storage, read> meshlet_vertex_positions: array<u32>; // Many per meshlet
@group(1) @binding(5) var<storage, read> meshlet_vertex_normals: array<u32>; // Many per meshlet
@group(1) @binding(6) var<storage, read> meshlet_vertex_uvs: array<vec2<f32>>; // Many per meshlet
@group(1) @binding(7) var<storage, read> meshlet_cluster_instance_ids: array<u32>; // Per cluster
@group(1) @binding(8) var<storage, read> meshlet_instance_uniforms: array<Mesh>; // Per entity instance

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

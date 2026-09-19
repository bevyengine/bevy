//! This shader is used for the compute_mesh example
//! The actual work it does is not important for the example and
//! has been hardcoded to return a cube mesh
import bevy_render::globals::Globals;

struct DataRanges {
    /// the starting offset of the mesh data in the *vertex_data* storage buffer
    vertex_start: u32,
    /// number of vertices
    num_vertices: u32,
    /// the starting offset of the index data in the *index_data* storage buffer
    index_start: u32,
    /// number of indices
    num_indices: u32,
}

@group(0) @binding(0) var<uniform> data_range: DataRanges;
@group(0) @binding(1) var<uniform> globals: Globals;
@group(0) @binding(2) var<storage, read_write> vertex_data: array<f32>;
@group(0) @binding(3) var<storage, read_write> index_data: array<u32>;

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // this loop is iterating over the full list of (position, normal, uv)
    // data what we have in `vertices`.
    //
    // The vertex_data buffer is bigger than just the mesh we're
    // processing because Bevy stores meshes in the mesh_allocator
    // which allocates slabs that each can contain multiple meshes.
    // This buffer is one slab, and data_range.vertex_start is the
    // starting offset for the mesh we care about.
    // So the 0 starting value in the for loop is added to
    // data_range.vertex_start which means we start writing at the
    // correct offset.
    //
    // The "end" of the available space to write into is known by us
    // ahead of time in this example, so we know this has enough space,
    // but you may wish to also check to make sure you are not writing
    // past the end of the range *because you should not write past the
    // end of the range ever*. Doing this can overwrite a different
    // mesh's data.
    for (var idx = 0u; idx < data_range.num_vertices; idx++) {
        // base indices for getting/setting vertices
        let get_base_idx = idx * vertex_stride;
        let set_base_idx = data_range.vertex_start + idx * vertex_stride;

        // we recalculate a time scale that is used to showcase that we can
        // recalculate the mesh attributes each frame. Be careful tho, for big
        // meshes this might result in performance drops
        let time_scale = (sin(vec3<f32>(globals.time, globals.time * 2.0, globals.time * 1.5)) + 1.0) * 0.5;

        // set the vertex position attribute (with the time scale to create an animation)
        let position = get_vec3(get_base_idx + position_offset) * time_scale;
        set_vec3(set_base_idx + position_offset, position);

        // set the vertex normal attribute
        let normal = get_vec3(get_base_idx + normal_offset);
        set_vec3(set_base_idx + normal_offset, normal);

        // set the vertex uv attribute
        let uv = get_vec2(get_base_idx + uv_offset);
        set_vec2(set_base_idx + uv_offset, uv);
    }

    for (var idx = 0u; idx < data_range.num_indices; idx++) {
        // This is doing the same as the vertex_data offset described above
        index_data[data_range.index_start + idx] = u32(indices[idx]);
    }
}

fn set_vec3(base_index: u32, data: vec3<f32>) {
    vertex_data[base_index + 0u] = data.x;
    vertex_data[base_index + 1u] = data.y;
    vertex_data[base_index + 2u] = data.z;
}

fn set_vec2(base_index: u32, data: vec2<f32>) {
    vertex_data[base_index + 0u] = data.x;
    vertex_data[base_index + 1u] = data.y;
}

fn get_vec3(base_index: u32) -> vec3<f32> {
    let x = vertices[base_index + 0u];
    let y = vertices[base_index + 1u];
    let z = vertices[base_index + 2u];
    return vec3<f32>(x,y,z);
}

fn get_vec2(base_index: u32) -> vec2<f32> {
    let x = vertices[base_index + 0u];
    let y = vertices[base_index + 1u];
    return vec2<f32>(x,y);
}

// hardcoded compute shader data.
/// half the size of the cube
const half_size = vec3(1.5);
const min = -half_size;
const max = half_size;

const vertex_stride = 8u;
const position_offset = 0u;
const normal_offset = 3u;
const uv_offset = 6u;

// Suppose Y-up right hand, and camera look from +Z to -Z
const vertices = array(
    // xyz, normal.xyz, uv.xy
    // Front
    min.x, min.y, max.z, 0.0, 0.0, 1.0, 0.0, 0.0,
    max.x, min.y, max.z, 0.0, 0.0, 1.0, 1.0, 0.0,
    max.x, max.y, max.z, 0.0, 0.0, 1.0, 1.0, 1.0,
    min.x, max.y, max.z, 0.0, 0.0, 1.0, 0.0, 1.0,
    // Back
    min.x, max.y, min.z, 0.0, 0.0, -1.0, 1.0, 0.0,
    max.x, max.y, min.z, 0.0, 0.0, -1.0, 0.0, 0.0,
    max.x, min.y, min.z, 0.0, 0.0, -1.0, 0.0, 1.0,
    min.x, min.y, min.z, 0.0, 0.0, -1.0, 1.0, 1.0,
    // Right
    max.x, min.y, min.z, 1.0, 0.0, 0.0, 0.0, 0.0,
    max.x, max.y, min.z, 1.0, 0.0, 0.0, 1.0, 0.0,
    max.x, max.y, max.z, 1.0, 0.0, 0.0, 1.0, 1.0,
    max.x, min.y, max.z, 1.0, 0.0, 0.0, 0.0, 1.0,
    // Left
    min.x, min.y, max.z, -1.0, 0.0, 0.0, 1.0, 0.0,
    min.x, max.y, max.z, -1.0, 0.0, 0.0, 0.0, 0.0,
    min.x, max.y, min.z, -1.0, 0.0, 0.0, 0.0, 1.0,
    min.x, min.y, min.z, -1.0, 0.0, 0.0, 1.0, 1.0,
    // Top
    max.x, max.y, min.z, 0.0, 1.0, 0.0, 1.0, 0.0,
    min.x, max.y, min.z, 0.0, 1.0, 0.0, 0.0, 0.0,
    min.x, max.y, max.z, 0.0, 1.0, 0.0, 0.0, 1.0,
    max.x, max.y, max.z, 0.0, 1.0, 0.0, 1.0, 1.0,
    // Bottom
    max.x, min.y, max.z, 0.0, -1.0, 0.0, 0.0, 0.0,
    min.x, min.y, max.z, 0.0, -1.0, 0.0, 1.0, 0.0,
    min.x, min.y, min.z, 0.0, -1.0, 0.0, 1.0, 1.0,
    max.x, min.y, min.z, 0.0, -1.0, 0.0, 0.0, 1.0
);

const indices = array(
    0, 1, 2, 2, 3, 0, // front
    4, 5, 6, 6, 7, 4, // back
    8, 9, 10, 10, 11, 8, // right
    12, 13, 14, 14, 15, 12, // left
    16, 17, 18, 18, 19, 16, // top
    20, 21, 22, 22, 23, 20, // bottom
);

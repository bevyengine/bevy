// This shader is used for the compute_mesh example
// The actual work it does is not important for the example and
// has been hardcoded to return a cube mesh

// `vertex` is the starting offset of the mesh data in the *vertex_data* storage buffer
// `vertex_index` is the starting offset of the *index* data in the *index_data* storage buffer
struct DataRanges {
    vertex_start: u32,
    vertex_end: u32,
    index_start: u32,
    index_end: u32,
}

@group(0) @binding(0) var<uniform> data_range: DataRanges;
@group(0) @binding(1) var<storage, read_write> vertex_data: array<f32>;
@group(0) @binding(2) var<storage, read_write> index_data: array<u32>;

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // this loop is iterating over the full list of (position, normal, uv)
    // data what we have in `vertices`.
    // `192` is used because arrayLength on const arrays doesn't work
    for (var i = 0u; i < 192; i++) {
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
        vertex_data[i + data_range.vertex_start] = vertices[i];
    }
    // `36` is the length of the `indices` array
    for (var i = 0u; i < 36; i++) {
        // This is doing the same as the vertex_data offset described above
        index_data[i + data_range.index_start] = u32(indices[i]);
    }
}

// hardcoded compute shader data.
// half_size is half the size of the cube
const half_size = vec3(1.5);
const min = -half_size;
const max = half_size;

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

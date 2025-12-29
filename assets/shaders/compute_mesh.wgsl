// This shader is used for the gpu_readback example
// The actual work it does is not important for the example

// This is the data that lives in the gpu only buffer
@group(0) @binding(0) var<storage, read_write> vertex_data: array<f32>;
@group(0) @binding(1) var<storage, read_write> index_data: array<u32>;

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // We use the global_id to index the array to make sure we don't
    // access data used in another workgroup
    // data[global_id.x] += 1u;
    for (var i = 0u; i < 192; i++) {
        vertex_data[i] = vertices[i];
    }
    for (var i = 0u; i < 36; i++) {
        index_data[i] = u32(indices[i]);
    }
    // data[0] = -min.x;
    // data[1] = min.y;
    // data[2] = max.z;
    // data[3] = 0.;
    // data[4] = 0.;
    // data[5] = 1.;
    // data[6] = 0.;
    // data[7] = 0.;
    // Write the same data to the texture
    // textureStore(texture, vec2<i32>(i32(global_id.x), 0), vec4<u32>(data[global_id.x], 0, 0, 0));
}

const half_size = vec3(2.);
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


// let positions: Vec<_> = vertices.iter().map(|(p, _, _)| *p).collect();
// let normals: Vec<_> = vertices.iter().map(|(_, n, _)| *n).collect();
// let uvs: Vec<_> = vertices.iter().map(|(_, _, uv)| *uv).collect();

const indices = array(
    0, 1, 2, 2, 3, 0, // front
    4, 5, 6, 6, 7, 4, // back
    8, 9, 10, 10, 11, 8, // right
    12, 13, 14, 14, 15, 12, // left
    16, 17, 18, 18, 19, 16, // top
    20, 21, 22, 22, 23, 20, // bottom
);

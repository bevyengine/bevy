// GPU mesh uniform building.
//
// This is a compute shader that expands each `MeshInputUniform` out to a full
// `MeshUniform` for each view before rendering. (Thus `MeshInputUniform`
// and `MeshUniform` are in a 1:N relationship.) It runs in parallel for all
// meshes for all views. As part of this process, the shader gathers each
// mesh's transform on the previous frame and writes it into the `MeshUniform`
// so that TAA works.

#import bevy_pbr::mesh_types::Mesh
#import bevy_render::maths

// Per-frame data that the CPU supplies to the GPU.
struct MeshInput {
    // The model transform.
    model: mat3x4<f32>,
    // The lightmap UV rect, packed into 64 bits.
    lightmap_uv_rect: vec2<u32>,
    // Various flags.
    flags: u32,
    // The index of this mesh's `MeshInput` in the `previous_input` array, if
    // applicable. If not present, this is `~0`.
    previous_input_index: u32,
}

// The current frame's `MeshInput`.
@group(0) @binding(0) var<storage> current_input: array<MeshInput>;
// The `MeshInput` values from the previous frame.
@group(0) @binding(1) var<storage> previous_input: array<MeshInput>;
// Indices into the `MeshInput` buffer.
//
// There may be many indices that map to the same `MeshInput`.
@group(0) @binding(2) var<storage> indices: array<u32>;
// The output array of `Mesh`es.
@group(0) @binding(3) var<storage, read_write> output: array<Mesh>;

@compute
@workgroup_size(64)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    let instance_index = global_invocation_id.x;
    if (instance_index >= arrayLength(&output)) {
        return;
    }

    // Unpack.
    let mesh_index = indices[instance_index];
    let model_affine_transpose = current_input[mesh_index].model;
    let model = maths::affine3_to_square(model_affine_transpose);

    // Calculate inverse transpose.
    let inverse_transpose_model = transpose(maths::inverse_affine3(transpose(
        model_affine_transpose)));

    // Pack inverse transpose.
    let inverse_transpose_model_a = mat2x4<f32>(
        vec4<f32>(inverse_transpose_model[0].xyz, inverse_transpose_model[1].x),
        vec4<f32>(inverse_transpose_model[1].yz, inverse_transpose_model[2].xy));
    let inverse_transpose_model_b = inverse_transpose_model[2].z;

    // Look up the previous model matrix.
    let previous_input_index = current_input[mesh_index].previous_input_index;
    var previous_model: mat3x4<f32>;
    if (previous_input_index == 0xffffffff) {
        previous_model = model_affine_transpose;
    } else {
        previous_model = previous_input[previous_input_index].model;
    }

    // Write the output.
    output[instance_index].model = model_affine_transpose;
    output[instance_index].previous_model = previous_model;
    output[instance_index].inverse_transpose_model_a = inverse_transpose_model_a;
    output[instance_index].inverse_transpose_model_b = inverse_transpose_model_b;
    output[instance_index].flags = current_input[mesh_index].flags;
    output[instance_index].lightmap_uv_rect = current_input[mesh_index].lightmap_uv_rect;
}

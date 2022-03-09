#import bevy_pbr::mesh_view_bind_group
#import bevy_pbr::mesh_struct

struct Vertex {
    [[location(0)]] i_model_col0: vec3<f32>;
    [[location(1)]] i_model_col1: vec3<f32>;
    [[location(2)]] i_model_col2: vec3<f32>;
    [[location(3)]] i_model_col3: vec3<f32>;
    [[location(4)]] i_inverse_model_col0: vec3<f32>;
    [[location(5)]] i_inverse_model_col1: vec3<f32>;
    [[location(6)]] i_inverse_model_col2: vec3<f32>;
    [[location(7), interpolate(flat)]] i_mesh_flags: u32;
    [[location(8)]] position: vec3<f32>;
    [[location(9)]] normal: vec3<f32>;
    [[location(10)]] uv: vec2<f32>;
#ifdef VERTEX_TANGENTS
    [[location(11)]] tangent: vec4<f32>;
#endif
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] mesh_flags: u32;
    [[location(1)]] world_position: vec4<f32>;
    [[location(2)]] world_normal: vec3<f32>;
    [[location(3)]] uv: vec2<f32>;
#ifdef VERTEX_TANGENTS
    [[location(4)]] world_tangent: vec4<f32>;
#endif
};

fn vec3_to_mat4x4(c0: vec3<f32>, c1: vec3<f32>, c2: vec3<f32>, c3: vec3<f32>) -> mat4x4<f32> {
    // NOTE: The shader compiler will optimize away multiplications by 0.0 and 1.0 and not use
    // registers for unused parts of the matrix
    return mat4x4<f32>(
        vec4<f32>(c0, 0.0),
        vec4<f32>(c1, 0.0),
        vec4<f32>(c2, 0.0),
        vec4<f32>(c3, 1.0)
    );
}

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
    let model = vec3_to_mat4x4(
        vertex.i_model_col0,
        vertex.i_model_col1,
        vertex.i_model_col2,
        vertex.i_model_col3
    );
    let world_position = model * vec4<f32>(vertex.position, 1.0);

    var out: VertexOutput;
    out.mesh_flags = vertex.i_mesh_flags;
    out.uv = vertex.uv;
    out.world_position = world_position;
    out.clip_position = view.view_proj * world_position;

    // NOTE: transpose is free on the GPU
    let inverse_transpose_model = transpose(mat3x3<f32>(
        vertex.i_inverse_model_col0,
        vertex.i_inverse_model_col1,
        vertex.i_inverse_model_col2,
    ));
    out.world_normal = inverse_transpose_model * vertex.normal;
#ifdef VERTEX_TANGENTS
    out.world_tangent = vec4<f32>(
        mat3x3<f32>(
            model[0].xyz,
            model[1].xyz,
            model[2].xyz
        ) * vertex.tangent.xyz,
        vertex.tangent.w
    );
#endif
    return out;
}

struct FragmentInput {
    [[builtin(front_facing)]] is_front: bool;
    [[location(0)]] mesh_flags: u32;
    [[location(1)]] world_position: vec4<f32>;
    [[location(2)]] world_normal: vec3<f32>;
    [[location(3)]] uv: vec2<f32>;
#ifdef VERTEX_TANGENTS
    [[location(4)]] world_tangent: vec4<f32>;
#endif
};

[[stage(fragment)]]
fn fragment(in: FragmentInput) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(1.0, 0.0, 1.0, 1.0);
}

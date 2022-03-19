#import bevy_pbr::mesh_view_bind_group
#import bevy_pbr::mesh_struct

struct Vertex {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
#ifdef VERTEX_TANGENTS
    [[location(3)]] tangent: vec4<f32>;
#endif
#ifdef SKINNED
    [[location(4)]] joint_indexes: vec4<u32>;
    [[location(5)]] joint_weights: vec4<f32>;
#endif
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] world_position: vec4<f32>;
    [[location(1)]] world_normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
#ifdef VERTEX_TANGENTS
    [[location(3)]] world_tangent: vec4<f32>;
#endif
};

[[group(2), binding(0)]]
var<uniform> mesh: Mesh;

#ifdef SKINNED
struct SkinnedMesh {
    data: array<mat4x4<f32>, 256u>;
};

[[group(3), binding(0)]]
var<uniform> joint_matrices: SkinnedMesh;

/// HACK: This works around naga not supporting matrix addition in SPIR-V 
// translations. See https://github.com/gfx-rs/naga/issues/1527
fn add_matrix(
    a: mat4x4<f32>,
    b: mat4x4<f32>,
) -> mat4x4<f32> {
    return mat4x4<f32>(
        a.x + b.x,
        a.y + b.y,
        a.z + b.z,
        a.w + b.w,
    );
}

fn skin_model(
    indexes: vec4<u32>,
    weights: vec4<f32>,
) -> mat4x4<f32> {
    var matrix = weights.x * joint_matrices.data[indexes.x];
    matrix = add_matrix(matrix, weights.y * joint_matrices.data[indexes.y]);
    matrix = add_matrix(matrix, weights.z * joint_matrices.data[indexes.z]);
    return add_matrix(matrix, weights.w * joint_matrices.data[indexes.w]);
}
#endif

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
#ifdef SKINNED
    var model = skin_model(vertex.joint_indexes, vertex.joint_weights);
    out.world_position = model * vec4<f32>(vertex.position, 1.0);
    out.world_normal = mat3x3<f32>(
        model[0].xyz,
        model[1].xyz,
        model[2].xyz
    ) * vertex.normal;
#else
    out.world_position = mesh.model * vec4<f32>(vertex.position, 1.0);
    out.world_normal = mat3x3<f32>(
        mesh.inverse_transpose_model[0].xyz,
        mesh.inverse_transpose_model[1].xyz,
        mesh.inverse_transpose_model[2].xyz
    ) * vertex.normal;
#endif

    // out.clip_position = view.view_proj * world_position;
    out.uv = vertex.uv;
    out.clip_position = view.view_proj * out.world_position;
#ifdef VERTEX_TANGENTS
    out.world_tangent = vec4<f32>(
        mat3x3<f32>(
            mesh.model[0].xyz,
            mesh.model[1].xyz,
            mesh.model[2].xyz
        ) * vertex.tangent.xyz,
        vertex.tangent.w
    );
#endif
    return out;
}

struct FragmentInput {
    [[builtin(front_facing)]] is_front: bool;
    [[location(0)]] world_position: vec4<f32>;
    [[location(1)]] world_normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
#ifdef VERTEX_TANGENTS
    [[location(3)]] world_tangent: vec4<f32>;
#endif
};

[[stage(fragment)]]
fn fragment(in: FragmentInput) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(1.0, 0.0, 1.0, 1.0);
}
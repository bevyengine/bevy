#ifdef SHADOW_MAPPING
#import bevy_pbr::view_struct
#else
#import bevy_pbr::mesh_view_bind_group
#endif
#import bevy_pbr::mesh_struct

struct Vertex {
    [[location(0)]] position: vec3<f32>;
#ifndef SHADOW_MAPPING
    [[location(1)]] normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
#ifdef VERTEX_TANGENTS
    [[location(3)]] tangent: vec4<f32>;
#endif
#endif
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
#ifndef SHADOW_MAPPING
    [[location(0)]] world_position: vec4<f32>;
    [[location(1)]] world_normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
#ifdef VERTEX_TANGENTS
    [[location(3)]] world_tangent: vec4<f32>;
#endif
#endif
};

#ifdef SHADOW_MAPPING
[[group(0), binding(0)]]
var<uniform> view: View;
[[group(1), binding(0)]]
var<uniform> mesh: Mesh;
#else
[[group(2), binding(0)]]
var<uniform> mesh: Mesh;
#endif

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
    let world_position = mesh.model * vec4<f32>(vertex.position, 1.0);

    var out: VertexOutput;
    out.clip_position = view.view_proj * world_position;
#ifndef SHADOW_MAPPING
    out.world_position = world_position;
    out.world_normal = mat3x3<f32>(
        mesh.inverse_transpose_model[0].xyz,
        mesh.inverse_transpose_model[1].xyz,
        mesh.inverse_transpose_model[2].xyz
    ) * vertex.normal;
    out.uv = vertex.uv;
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
#endif
    return out;
}

struct FragmentInput {
    [[builtin(front_facing)]] is_front: bool;
#ifndef SHADOW_MAPPING
    [[location(0)]] world_position: vec4<f32>;
    [[location(1)]] world_normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
#ifdef VERTEX_TANGENTS
    [[location(3)]] world_tangent: vec4<f32>;
#endif
#endif
};

[[stage(fragment)]]
fn fragment(in: FragmentInput) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(1.0, 0.0, 1.0, 1.0);
}

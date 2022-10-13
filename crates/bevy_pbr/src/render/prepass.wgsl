#import bevy_pbr::mesh_view_types
#import bevy_pbr::mesh_types

@group(0) @binding(0)
var<uniform> view: View;

@group(1) @binding(0)
var<uniform> mesh: Mesh;

#ifdef SKINNED
@group(1) @binding(1)
var<uniform> joint_matrices: SkinnedMesh;
#import bevy_pbr::skinning
#endif

// NOTE: Bindings must come before functions that use them!
#import bevy_pbr::mesh_functions

struct Vertex {
    @location(0) position: vec3<f32>,

#ifdef OUTPUT_NORMALS
    @location(1) normal: vec3<f32>,
#ifdef VERTEX_UVS
    @location(2) uv: vec2<f32>,
#endif // VERTEX_UVS
#ifdef VERTEX_TANGENTS
    @location(3) tangent: vec4<f32>,
#endif // VERTEX_TANGENTS
#endif // OUTPUT_NORMALS

#ifdef SKINNED
    @location(4) joint_indices: vec4<u32>,
    @location(5) joint_weights: vec4<f32>,
#endif // SKINNED
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,

#ifdef OUTPUT_NORMALS
    @location(0) world_normal: vec3<f32>,
#ifdef VERTEX_UVS
    @location(1) uv: vec2<f32>,
#endif // VERTEX_UVS
#ifdef VERTEX_TANGENTS
    @location(2) world_tangent: vec4<f32>,
#endif // VERTEX_TANGENTS
#endif // OUTPUT_NORMALS
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

#ifdef SKINNED
    var model = skin_model(vertex.joint_indices, vertex.joint_weights);
#else // SKINNED
    var model = mesh.model;
#endif // SKINNED

    out.clip_position = mesh_position_local_to_clip(model, vec4<f32>(vertex.position, 1.0));

#ifdef OUTPUT_NORMALS
#ifdef SKINNED
    out.world_normal = skin_normals(model, vertex.normal);
#else // SKINNED
    out.world_normal = mesh_normal_local_to_world(vertex.normal);
#endif // SKINNED

#ifdef VERTEX_UVS
    out.uv = vertex.uv;
#endif // VERTEX_UVS

#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_tangent_local_to_world(model, vertex.tangent);
#endif // VERTEX_TANGENTS
#endif // OUTPUT_NORMALS

    return out;
}

#ifdef OUTPUT_NORMALS
struct FragmentInput {
    @builtin(front_facing) is_front: bool,
    @location(0) world_normal: vec3<f32>,
#ifdef VERTEX_UVS
    @location(1) uv: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(2) world_tangent: vec4<f32>,
#endif
}

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.world_normal * 0.5 + vec3<f32>(0.5), 1.0);
}
#endif // OUTPUT_NORMALS

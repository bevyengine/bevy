#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::mesh_bindings as MeshBindings
#import bevy_pbr::mesh_functions as MeshFunctions
#import bevy_pbr::skinning as Skinning
#import bevy_pbr::mesh_vertex_output as OutputTypes

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
#ifdef VERTEX_UVS
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(3) tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(4) color: vec4<f32>,
#endif
#ifdef SKINNED
    @location(5) joint_indices: vec4<u32>,
    @location(6) joint_weights: vec4<f32>,
#endif
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    // have to copy-paste here, we can't currently embed an unlocated struct in the vertex stage output
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    #ifdef VERTEX_UVS
    @location(2) uv: vec2<f32>,
    #endif
    #ifdef VERTEX_TANGENTS
    @location(3) world_tangent: vec4<f32>,
    #endif
    #ifdef VERTEX_COLORS
    @location(4) color: vec4<f32>,
    #endif
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
#ifdef SKINNED
    var model = Skinning::skin_model(vertex.joint_indices, vertex.joint_weights);
    out.world_normal = Skinning::skin_normals(model, vertex.normal);
#else
    var model = MeshBindings::mesh.model;
    out.world_normal = MeshFunctions::mesh_normal_local_to_world(vertex.normal);
#endif
    out.world_position = MeshFunctions::mesh_position_local_to_world(model, vec4<f32>(vertex.position, 1.0));
#ifdef VERTEX_UVS
    out.uv = vertex.uv;
#endif
#ifdef VERTEX_TANGENTS
    out.world_tangent = MeshFunctions::mesh_tangent_local_to_world(model, vertex.tangent);
#endif
#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif

    out.clip_position = MeshFunctions::mesh_position_world_to_clip(out.world_position);
    return out;
}

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: OutputTypes::MeshVertexOutput,
) -> @location(0) vec4<f32> {
#ifdef VERTEX_COLORS
    return mesh.color;
#else
    return vec4<f32>(1.0, 0.0, 1.0, 1.0);
#endif
}

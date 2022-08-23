#import bevy_sprite::mesh2d_bindings
#import bevy_sprite::mesh2d_functions as mesh_functions
#import bevy_sprite::mesh2d_vertex_output

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
#ifdef VERTEX_TANGENTS
    @location(3) tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(4) color: vec4<f32>,
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
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
#ifdef VERTEX_UVS
    out.uv = vertex.uv;
#endif
    out.world_position = mesh_functions::mesh2d_position_local_to_world(
        bevy_sprite::mesh2d_bindings::mesh.model, 
        vec4<f32>(vertex.position, 1.0)
    );
    out.clip_position = mesh_functions::mesh2d_position_world_to_clip(out.world_position);
    out.world_normal = mesh_functions::mesh2d_normal_local_to_world(vertex.normal);
#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh2d_tangent_local_to_world(vertex.tangent);
#endif
#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif
    return out;
}

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: bevy_sprite::mesh2d_vertex_output::MeshVertexOutput,
) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 1.0, 1.0);
}

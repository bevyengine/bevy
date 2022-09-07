#import bevy_sprite::mesh2d_view_bindings
#import bevy_sprite::mesh2d_bindings

// NOTE: Bindings must come before functions that use them!
#import bevy_sprite::mesh2d_functions

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    #import bevy_sprite::mesh2d_vertex_output
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex.uv;
    out.clip_position = vec4<f32>((out.uv - vec2<f32>(0.5, 0.5)) * 2.0, 0.0, 1.0);
    return out;
}
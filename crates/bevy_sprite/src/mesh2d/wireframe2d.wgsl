#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct PushConstants {
    color: vec4<f32>
}

var<push_constant> push_constants: PushConstants;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return push_constants.color;
}

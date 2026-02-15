#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct PushConstants {
    color: vec4<f32>
}

var<immediate> push_constants: PushConstants;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return push_constants.color;
}

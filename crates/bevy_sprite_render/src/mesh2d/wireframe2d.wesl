import package::mesh2d::vertex_output::VertexOutput;
import bevy_render::writer_encode::writer_encode;

struct PushConstants {
    color: vec4<f32>
}

var<immediate> push_constants: PushConstants;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return writer_encode(push_constants.color);
}

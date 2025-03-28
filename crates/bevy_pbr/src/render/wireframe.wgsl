#import bevy_pbr::{
    mesh_bindings::mesh,
    mesh_functions,
    skinning,
    morph::morph,
    forward_io::{Vertex, VertexOutput},
    view_transformations::position_world_to_clip,
}

struct PushConstants {
    color: vec4<f32>
}

var<push_constant> push_constants: PushConstants;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return push_constants.color;
}

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct WireframeMaterial {
    color: vec4<f32>,
};

@group(2) @binding(0) var<uniform> material: WireframeMaterial;
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return material.color;
}

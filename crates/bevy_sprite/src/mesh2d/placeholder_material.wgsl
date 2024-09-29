#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct PlaceholderMaterial2d {
    color: vec4<f32>,
};

@group(2) @binding(0) var<uniform> material: PlaceholderMaterial2d;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    return material.color;
}

#import bevy_pbr::mesh_view_bindings as ViewBindings
#import bevy_pbr::mesh_vertex_output as OutputTypes

@group(1) @binding(0)
var texture: texture_2d<f32>;

@group(1) @binding(1)
var our_sampler: sampler;

@fragment
fn fragment(
    @builtin(position) position: vec4<f32>,
    mesh: OutputTypes::MeshVertexOutput
) -> @location(0) vec4<f32> {
    // Get screen position with coordinates from 0 to 1
    let uv = position.xy / vec2<f32>(ViewBindings::view.width, ViewBindings::view.height);
    let offset_strength = 0.02;

    // Sample each color channel with an arbitrary shift
    var output_color = vec4<f32>(
        textureSample(texture, our_sampler, uv + vec2<f32>(offset_strength, -offset_strength)).r,
        textureSample(texture, our_sampler, uv + vec2<f32>(-offset_strength, 0.0)).g,
        textureSample(texture, our_sampler, uv + vec2<f32>(0.0, offset_strength)).b,
        1.0
        );

    return output_color;
}

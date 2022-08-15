#import bevy_pbr::mesh_view_bindings as ViewBindings
#import bevy_pbr::mesh_vertex_output as OutputTypes

@group(1) @binding(0)
var texture: texture_2d<f32>;
@group(1) @binding(1)
var texture_sampler: sampler;

@fragment
fn fragment(
    @builtin(position) position: vec4<f32>,
    mesh: OutputTypes::MeshVertexOutput,
) -> @location(0) vec4<f32> {
    let uv = position.xy / vec2<f32>(ViewBindings::view.width, ViewBindings::view.height);
    let color = textureSample(texture, texture_sampler, uv);
    return color;
}

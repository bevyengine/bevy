#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::mesh_vertex_output

@group(1) @binding(0)
var texture: texture_2d<f32>;
@group(1) @binding(1)
var texture_sampler: sampler;

@fragment
fn fragment(
    mesh: bevy_pbr::mesh_vertex_output::MeshVertexOutput,
) -> @location(0) vec4<f32> {
    let view = bevy_pbr::mesh_view_bindings::view;
    let uv = mesh.clip_position.xy / vec2<f32>(view.width, view.height);
    let color = textureSample(texture, texture_sampler, uv);
    return color;
}

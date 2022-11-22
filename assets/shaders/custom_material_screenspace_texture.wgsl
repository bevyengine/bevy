#import bevy_render::core_bindings
#import bevy_pbr::mesh_vertex_output
#import bevy_pbr::utils

@group(1) @binding(0)
var texture: texture_2d<f32>;
@group(1) @binding(1)
var texture_sampler: sampler;

@fragment
fn fragment(
    mesh: bevy_pbr::mesh_vertex_output::MeshVertexOutput,
) -> @location(0) vec4<f32> {
    let view = bevy_render::core_bindings::view;
    let uv = bevy_pbr::utils::coords_to_viewport_uv(mesh.clip_position.xy, view.viewport);
    let color = textureSample(texture, texture_sampler, uv);
    return color;
}

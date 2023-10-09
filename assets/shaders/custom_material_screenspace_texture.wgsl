#import bevy_pbr::mesh_view_bindings  view
#import bevy_pbr::mesh_vertex_output  MeshVertexOutput
#import bevy_pbr::utils               coords_to_viewport_uv

@group(1) @binding(0) var texture: texture_2d<f32>;
@group(1) @binding(1) var texture_sampler: sampler;

@fragment
fn fragment(
    mesh: MeshVertexOutput,
) -> @location(0) vec4<f32> {
    let viewport_uv = coords_to_viewport_uv(mesh.position.xy, view.viewport);
    let color = textureSample(texture, texture_sampler, viewport_uv);
    return color;
}

#from bevy_pbr::mesh_view_bindings  import view
#from bevy_pbr::mesh_vertex_output  import MeshVertexOutput
#from bevy_pbr::utils               import coords_to_viewport_uv

@group(1) @binding(0)
var texture: texture_2d<f32>;
@group(1) @binding(1)
var texture_sampler: sampler;

@fragment
fn fragment(
    mesh: ::MeshVertexOutput,
) -> @location(0) vec4<f32> {
    let view = ::view;
    let uv = ::coords_to_viewport_uv(mesh.clip_position.xy, view.viewport);
    let color = textureSample(texture, texture_sampler, uv);
    return color;
}

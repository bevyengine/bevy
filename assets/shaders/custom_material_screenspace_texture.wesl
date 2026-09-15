import bevy_pbr::render::{
    mesh_view_bindings::view,
    forward_io::VertexOutput,
    utils::coords_to_viewport_uv,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(0) public var texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) public var texture_sampler: sampler;

@fragment
public fn fragment(
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
    let viewport_uv = coords_to_viewport_uv(mesh.position.xy, view.viewport);
    let color = textureSample(texture, texture_sampler, viewport_uv);
    return color;
}

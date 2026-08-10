import bevy_pbr::render::forward_io::VertexOutput;

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var material_color_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var material_color_sampler: sampler;

@fragment
fn fragment(
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
    return textureSample(material_color_texture, material_color_sampler, mesh.uv);
}

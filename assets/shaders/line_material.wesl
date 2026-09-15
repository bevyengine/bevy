import bevy_pbr::render::forward_io::VertexOutput;

public struct LineMaterial {
    color: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(0) public var<uniform> material: LineMaterial;

@fragment
public fn fragment(
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
    return material.color;
}

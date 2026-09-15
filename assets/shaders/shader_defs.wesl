import bevy_pbr::render::forward_io::VertexOutput;

public struct CustomMaterial {
    color: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(0) public var<uniform> material: CustomMaterial;

@fragment
public fn fragment(
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
    @if(IS_RED)
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
    @else
    return material.color;
}

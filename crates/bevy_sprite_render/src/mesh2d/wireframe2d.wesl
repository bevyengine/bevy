import package::mesh2d::vertex_output::VertexOutput;

public struct PushConstants {
    color: vec4<f32>
}

public var<immediate> push_constants: PushConstants;

@fragment
public fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return push_constants.color;
}

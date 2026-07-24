// This shader draws the color plane in various color spaces.
import bevy_ui_render::ui_vertex_output::UiVertexOutput;
import bevy_render::color_operations::{
    srgb_to_linear_rgb,
    hsl_to_linear_rgb,
};

struct ColorPlaneUniform {
  fixed_channel : f32,
@if(SIXTEEN_BYTE_ALIGNMENT)
  _webgl2_padding_12b : vec3<f32>,
}

@group(1) @binding(0) var<uniform> uniform_data : ColorPlaneUniform;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
@if(PLANE_RG)
    return vec4(srgb_to_linear_rgb(vec3(uv.x, uv.y, uniform_data.fixed_channel)), 1.0);
@elif(PLANE_RB)
    return vec4(srgb_to_linear_rgb(vec3(uv.x, uniform_data.fixed_channel, uv.y)), 1.0);
@elif(PLANE_GB)
    return vec4(srgb_to_linear_rgb(vec3(uniform_data.fixed_channel, uv.x, uv.y)), 1.0);
@elif(PLANE_HS)
    return vec4(hsl_to_linear_rgb(vec3(uv.x, 1.0 - uv.y, uniform_data.fixed_channel)), 1.0);
@elif(PLANE_HL)
    return vec4(hsl_to_linear_rgb(vec3(uv.x, uniform_data.fixed_channel, 1.0 - uv.y)), 1.0);
@else
    // Error color
    return vec4(1.0, 0.0, 1.0, 1.0);
}

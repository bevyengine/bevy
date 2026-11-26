//! This shader draws the color plane in various color spaces.
import bevy_ui_render::ui_vertex_output::UiVertexOutput;
import bevy_render::color_operations::{
    srgb_to_linear_rgb,
    hsl_to_linear_rgb,
    okhsl_to_linear_rgb,
};
import bevy_render::writer_encode::writer_encode;

struct ColorPlaneUniform {
    fixed_channel: f32,
@if(SIXTEEN_BYTE_ALIGNMENT)
    _webgl2_padding_12b: vec3<f32>,
}

@group(1) @binding(0) var<uniform> uniform_data: ColorPlaneUniform;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    var rgb: vec3<f32>;
@if(PLANE_RG)
    rgb = srgb_to_linear_rgb(vec3(uv.x, 1.0 - uv.y, uniform_data.fixed_channel));
@elif(PLANE_RB)
    rgb = srgb_to_linear_rgb(vec3(uv.x, uniform_data.fixed_channel, 1.0 - uv.y));
@elif(PLANE_GB)
    rgb = srgb_to_linear_rgb(vec3(uniform_data.fixed_channel, uv.x, 1.0 - uv.y));
@elif(PLANE_HS)
    rgb = hsl_to_linear_rgb(vec3(uv.x, 1.0 - uv.y, uniform_data.fixed_channel));
@elif(PLANE_HL)
    rgb = hsl_to_linear_rgb(vec3(uv.x, uniform_data.fixed_channel, 1.0 - uv.y));
@elif(PLANE_OKHS)
    rgb = okhsl_to_linear_rgb(vec3(uv.x, 1.0 - uv.y, uniform_data.fixed_channel));
@elif(PLANE_OKHL)
    rgb = okhsl_to_linear_rgb(vec3(uv.x, uniform_data.fixed_channel, 1.0 - uv.y));
@else
    // Error color
    rgb = vec3(1.0, 0.0, 1.0);

    return writer_encode(vec4(rgb, 1.0));
}

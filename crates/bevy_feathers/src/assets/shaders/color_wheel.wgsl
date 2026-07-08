// This shader draws the color wheel in various color spaces.
#import bevy_ui::ui_vertex_output::UiVertexOutput
#import bevy_render::color_operations::{
    hsv_to_linear_rgb,
    hsl_to_linear_rgb,
}
#import bevy_render::maths::PI_2

struct ColorPlaneUniform {
  fixed_channel : f32,
#ifdef SIXTEEN_BYTE_ALIGNMENT
  _webgl2_padding_12b : vec3<f32>,
#endif
}

@group(1) @binding(0) var<uniform> uniform_data : ColorPlaneUniform;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let min_side = min(in.size.x, in.size.y);
    let diameters = min_side / in.size;
    // UV space is from -0.5 to 0.5
    let centered = (in.uv - vec2(0.5)) / diameters;
    let radial = length(centered);

    let hue = fract(atan2(centered.y, centered.x) / PI_2);
    let saturation = radial * 2.0;
    let hsx = vec3(hue, saturation, uniform_data.fixed_channel);

    let aa = fwidth(radial);
    let alpha = 1.0 - smoothstep(0.5 - aa, 0.5, radial);

#ifdef WHEEL_HSL
    return vec4(hsl_to_linear_rgb(hsx), alpha);
#else ifdef WHEEL_HSV
    return vec4(hsv_to_linear_rgb(hsx), alpha);
#else
    // Error color
    return vec4(1.0, 0.0, 1.0, alpha);
#endif
}

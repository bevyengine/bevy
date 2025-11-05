// This shader draws the color plane in various color spaces.
#import bevy_ui::ui_vertex_output::UiVertexOutput
#import bevy_ui_render::color_space::{
    srgb_to_linear_rgb,
    hsl_to_linear_rgb,
}

@group(1) @binding(0) var<uniform> fixed_channel: f32;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
#ifdef PLANE_RG
    return vec4<f32>(srgb_to_linear_rgb(vec3<f32>(uv.x, uv.y, fixed_channel)), 1.0);
#else ifdef PLANE_RB
    return vec4<f32>(srgb_to_linear_rgb(vec3<f32>(uv.x, fixed_channel, uv.y)), 1.0);
#else ifdef PLANE_GB
    return vec4<f32>(srgb_to_linear_rgb(vec3<f32>(fixed_channel, uv.x, uv.y)), 1.0);
#else ifdef PLANE_HS
    return vec4<f32>(hsl_to_linear_rgb(vec3<f32>(uv.x, 1.0 - uv.y, fixed_channel)), 1.0);
#else ifdef PLANE_HL
    return vec4<f32>(hsl_to_linear_rgb(vec3<f32>(uv.x, fixed_channel, 1.0 - uv.y)), 1.0);
#else
    // Error color
    return vec4<f32>(1.0, 0.0, 1.0, 1.0);
#endif
}

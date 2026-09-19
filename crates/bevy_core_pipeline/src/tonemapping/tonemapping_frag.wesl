import bevy_render::{
    view::View,
    maths::powsafe,
};
import package::{
    fullscreen_vertex_shader::FullscreenVertexOutput,
    tonemapping::{tone_mapping, screen_space_dither},
};
@if(COMPOSITING_SPACE_SRGB)
import bevy_render::color_operations::{srgb_to_linear, linear_to_srgb};
@if(COMPOSITING_SPACE_OKLAB)
import bevy_render::color_operations::{oklab_to_linear_rgb, linear_rgb_to_oklab};

@group(0) @binding(0) var<uniform> view: View;

@group(0) @binding(1) var hdr_texture: texture_2d<f32>;
@group(0) @binding(2) var hdr_sampler: sampler;
@group(0) @binding(3) var dt_lut_texture: texture_3d<f32>;
@group(0) @binding(4) var dt_lut_sampler: sampler;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let hdr_color = textureSample(hdr_texture, hdr_sampler, in.uv);

    // When the view composites in an encoded space, the main pass writes encoded
    // values, so decode before tonemapping and re-encode after.
@if(COMPOSITING_SPACE_SRGB)
    var output_rgb = tone_mapping(vec4(srgb_to_linear(hdr_color.rgb), hdr_color.a), view.color_grading).rgb;
@elif(COMPOSITING_SPACE_OKLAB)
    var output_rgb = tone_mapping(vec4(oklab_to_linear_rgb(hdr_color.rgb), hdr_color.a), view.color_grading).rgb;
@else
    var output_rgb = tone_mapping(hdr_color, view.color_grading).rgb;

@if(DEBAND_DITHER) {
    output_rgb = powsafe(output_rgb.rgb, 1.0 / 2.2);
    output_rgb = output_rgb + screen_space_dither(in.position.xy);
    // This conversion back to linear space is required because our output texture format is
    // SRGB; the GPU will assume our output is linear and will apply an SRGB conversion.
    output_rgb = powsafe(output_rgb.rgb, 2.2);
}

@if(COMPOSITING_SPACE_SRGB) {
    output_rgb = linear_to_srgb(output_rgb);
}
@if(COMPOSITING_SPACE_OKLAB) {
    output_rgb = linear_rgb_to_oklab(output_rgb);
}

    return vec4<f32>(output_rgb, hdr_color.a);
}

#define TONEMAPPING_PASS

#import bevy_core_pipeline::fullscreen_vertex_shader  FullscreenVertexOutput
#import bevy_render::view  View
#import bevy_core_pipeline::tonemapping  tone_mapping, powsafe, screen_space_dither

@group(0) @binding(0) var<uniform> view: View;

@group(0) @binding(1) var hdr_texture: texture_2d<f32>;
@group(0) @binding(2) var hdr_sampler: sampler;
@group(0) @binding(3) var dt_lut_texture: texture_3d<f32>;
@group(0) @binding(4) var dt_lut_sampler: sampler;

#import bevy_core_pipeline::tonemapping

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let hdr_color = textureSample(hdr_texture, hdr_sampler, in.uv);

    var output_rgb = tone_mapping(hdr_color, view.color_grading).rgb;

#ifdef DEBAND_DITHER
    output_rgb = powsafe(output_rgb.rgb, 1.0 / 2.2);
    output_rgb = output_rgb + bevy_core_pipeline::tonemapping::screen_space_dither(in.position.xy);
    // This conversion back to linear space is required because our output texture format is
    // SRGB; the GPU will assume our output is linear and will apply an SRGB conversion.
    output_rgb = powsafe(output_rgb.rgb, 2.2);
#endif

    return vec4<f32>(output_rgb, hdr_color.a);
}

#import bevy_core_pipeline::fullscreen_vertex_shader
#import bevy_core_pipeline::tonemapping

@group(0) @binding(0)
var hdr_texture: texture_2d<f32>;
@group(0) @binding(1)
var hdr_sampler: sampler;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let hdr_color = textureSample(hdr_texture, hdr_sampler, in.uv);

    var output_rgb = reinhard_luminance(hdr_color.rgb);

#ifdef DEBAND_DITHER
    output_rgb = pow(output_rgb.rgb, vec3<f32>(1.0 / 2.2));
    output_rgb = output_rgb + screen_space_dither(in.position.xy);
    // This conversion back to linear space is required because our output texture format is
    // SRGB; the GPU will assume our output is linear and will apply an SRGB conversion.
    output_rgb = pow(output_rgb.rgb, vec3<f32>(2.2));
#endif

    return vec4<f32>(output_rgb, hdr_color.a);
}

#import bevy_core_pipeline::fullscreen_vertex_shader
#import bevy_core_pipeline::tonemapping

@group(0) @binding(0)
var hdr_texture: texture_2d<f32>;
@group(0) @binding(1)
var hdr_sampler: sampler;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let hdr_color = textureSample(hdr_texture, hdr_sampler, in.uv);

    var out = reinhard_luminance(hdr_color.rgb);

#ifdef DEBAND_DITHER
    out = vec3(to_srgb(out.r), to_srgb(out.g), to_srgb(out.b));
    out = out + screen_space_dither(in.position.xy);
    // This conversion back to linear space is required because our output texture format is
    // SRGB; the GPU will assume our output is linear and will apply an SRGB conversion.
    out = vec3(to_linear(out.r), to_linear(out.g), to_linear(out.b));
#endif

    return vec4<f32>(out, hdr_color.a);
}

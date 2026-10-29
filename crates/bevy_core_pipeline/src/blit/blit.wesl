import package::fullscreen_vertex_shader::FullscreenVertexOutput;
@if(SRGB_TO_LINEAR) import bevy_render::color_operations::srgb_to_linear;
@if(OKLAB_TO_LINEAR) import bevy_render::color_operations::oklab_to_linear_rgb;
@if(DISPLAY_ENCODE) import bevy_render::transfer_functions::{scrgb_encode, pq_inverse_eotf_from_nits, srgb_oetf_extended};

@if(DISPLAY_ENCODE)
struct DisplayTargetUniform {
    gamut: mat3x3<f32>,
    paper_white_nits: f32,
}

@group(0) @binding(0) var in_texture: texture_2d<f32>;
@group(0) @binding(1) var in_sampler: sampler;
@if(DISPLAY_ENCODE)
@group(1) @binding(0) var<uniform> display_target: DisplayTargetUniform;

@fragment
fn fs_main(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(in_texture, in_sampler, in.uv);
    @if(SRGB_TO_LINEAR)
    color = vec4(srgb_to_linear(color.rgb), color.a);
    @if(OKLAB_TO_LINEAR)
    color = vec4(oklab_to_linear_rgb(color.rgb), color.a);
@if(DISPLAY_ENCODE) {
    var rgb = display_target.gamut * color.rgb;

    let paper_white = display_target.paper_white_nits;
    @if(DISPLAY_TRANSFER_PQ)
    rgb = pq_inverse_eotf_from_nits(rgb * paper_white);
    @else
    rgb = scrgb_encode(rgb, paper_white);
    @if(DISPLAY_TRANSFER_EXTENDED_SRGB)
    rgb = srgb_oetf_extended(rgb);
    color = vec4(rgb, color.a);
}
    return color;
}

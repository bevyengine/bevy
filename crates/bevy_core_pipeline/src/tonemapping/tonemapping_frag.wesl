import bevy_render::{
    view::View,
    maths::powsafe,
};
import bevy_core_pipeline::{
    fullscreen_vertex_shader::FullscreenVertexOutput,
    tonemapping::{tone_mapping, screen_space_dither},
};

// View uniform, shaped the same as `bevy_pbr::render::mesh_view_bindings::view_array`
// so the tonemapping pipeline can share the packed
// `DynamicArrayUniformBuffer` behind `ViewUniforms`. The shader reads through
// `view_array[current_view_index]`, which indexes the array at `current_view_index` (set from
// `@builtin(view_index)` at the top of the fragment under MULTIVIEW). For
// non-multiview pipelines, `MAX_VIEW_COUNT` is undefined and the fallback
// `array<View, 1>` matches the single `ViewUniform` packed per camera.
@if(MAX_VIEW_COUNT)
@group(0) @binding(0) var<uniform> view_array: array<View, constants::MAX_VIEW_COUNT>;
@else
@group(0) @binding(0) var<uniform> view_array: array<View, 1>;
var<private> current_view_index: u32 = 0u;


@group(0) @binding(1) var hdr_texture: texture_2d<f32>;
@group(0) @binding(2) var hdr_sampler: sampler;
@group(0) @binding(3) var dt_lut_texture: texture_3d<f32>;
@group(0) @binding(4) var dt_lut_sampler: sampler;

@fragment
fn fragment(
    in: FullscreenVertexOutput,
@if(MULTIVIEW)
    @builtin(view_index) view_index: u32,
) -> @location(0) vec4<f32> {
@if(MULTIVIEW)
    current_view_index = view_index;
    let hdr_color = textureSample(hdr_texture, hdr_sampler, in.uv);

    var output_rgb = tone_mapping(hdr_color, view_array[current_view_index].color_grading).rgb;

@if(DEBAND_DITHER) {
    output_rgb = powsafe(output_rgb.rgb, 1.0 / 2.2);
    output_rgb = output_rgb + screen_space_dither(in.position.xy);
    // This conversion back to linear space is required because our output texture format is
    // SRGB; the GPU will assume our output is linear and will apply an SRGB conversion.
    output_rgb = powsafe(output_rgb.rgb, 2.2);
}

    return vec4<f32>(output_rgb, hdr_color.a);
}

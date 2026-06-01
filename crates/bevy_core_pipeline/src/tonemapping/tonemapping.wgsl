#define TONEMAPPING_PASS

#import bevy_render::{
    view::View,
    maths::powsafe,
}
#import bevy_core_pipeline::{
    fullscreen_vertex_shader::FullscreenVertexOutput,
    tonemapping::{tone_mapping, screen_space_dither},
}

// View uniform, shaped the same as `bevy_pbr::mesh_view_bindings::view_array`
// so the tonemapping pipeline can share the packed
// `DynamicArrayUniformBuffer` behind `ViewUniforms`. The shader reads through
// `view()`, which indexes the array at `current_view_index` (set from
// `@builtin(view_index)` at the top of the fragment under MULTIVIEW). For
// non-multiview pipelines, `MAX_VIEW_COUNT` is undefined and the fallback
// `array<View, 1>` matches the single `ViewUniform` packed per camera.
#ifdef MAX_VIEW_COUNT
@group(0) @binding(0) var<uniform> view_array: array<View, #{MAX_VIEW_COUNT}>;
#else
@group(0) @binding(0) var<uniform> view_array: array<View, 1>;
#endif
var<private> current_view_index: i32 = 0;

fn view() -> View {
    return view_array[current_view_index];
}

@group(0) @binding(1) var hdr_texture: texture_2d<f32>;
@group(0) @binding(2) var hdr_sampler: sampler;
@group(0) @binding(3) var dt_lut_texture: texture_3d<f32>;
@group(0) @binding(4) var dt_lut_sampler: sampler;

@fragment
fn fragment(
    in: FullscreenVertexOutput,
#ifdef MULTIVIEW
    @builtin(view_index) view_index: i32,
#endif
) -> @location(0) vec4<f32> {
#ifdef MULTIVIEW
    current_view_index = view_index;
#endif
    let hdr_color = textureSample(hdr_texture, hdr_sampler, in.uv);

    var output_rgb = tone_mapping(hdr_color, view().color_grading).rgb;

#ifdef DEBAND_DITHER
    output_rgb = powsafe(output_rgb.rgb, 1.0 / 2.2);
    output_rgb = output_rgb + screen_space_dither(in.position.xy);
    // This conversion back to linear space is required because our output texture format is
    // SRGB; the GPU will assume our output is linear and will apply an SRGB conversion.
    output_rgb = powsafe(output_rgb.rgb, 2.2);
#endif

    return vec4<f32>(output_rgb, hdr_color.a);
}

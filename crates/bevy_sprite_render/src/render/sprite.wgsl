#ifdef TONEMAP_IN_SHADER
#import bevy_core_pipeline::tonemapping
#endif

#import bevy_render::{
    maths::affine3_to_square,
    view::View,
}

#import bevy_sprite::sprite_view_bindings::view

const TEXT_EFFECT_TEXT: u32 = 1u;
const TEXT_EFFECT_SHADOW: u32 = 2u;
const TEXT_EFFECT_OUTLINE: u32 = 4u;

struct VertexInput {
    @builtin(vertex_index) index: u32,
    // NOTE: Instance-rate vertex buffer members prefixed with i_
    // NOTE: i_model_transpose_colN are the 3 columns of a 3x4 matrix that is the transpose of the
    // affine 4x3 model matrix.
    @location(0) i_model_transpose_col0: vec4<f32>,
    @location(1) i_model_transpose_col1: vec4<f32>,
    @location(2) i_model_transpose_col2: vec4<f32>,
    @location(3) i_color: vec4<f32>,
    @location(4) i_uv_offset_scale: vec4<f32>,
    @location(5) i_effect_params: vec4<f32>,
    @location(6) i_shadow_color: vec4<f32>,
    @location(7) i_outline_color: vec4<f32>,
    @location(8) i_effect_flags: vec4<u32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) color: vec4<f32>,
    @location(2) @interpolate(flat) shadow_color: vec4<f32>,
    @location(3) @interpolate(flat) outline_color: vec4<f32>,
    @location(4) @interpolate(flat) effect_params: vec4<f32>,
    @location(5) @interpolate(flat) effect_flags: u32,
};

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let vertex_position = vec3<f32>(
        f32(in.index & 0x1u),
        f32((in.index & 0x2u) >> 1u),
        0.0
    );

    out.clip_position = view.clip_from_world * affine3_to_square(mat3x4<f32>(
        in.i_model_transpose_col0,
        in.i_model_transpose_col1,
        in.i_model_transpose_col2,
    )) * vec4<f32>(vertex_position, 1.0);
    out.uv = vec2<f32>(vertex_position.xy) * in.i_uv_offset_scale.zw + in.i_uv_offset_scale.xy;
    out.color = in.i_color;
    out.shadow_color = in.i_shadow_color;
    out.outline_color = in.i_outline_color;
    out.effect_params = in.i_effect_params;
    out.effect_flags = in.i_effect_flags.x;

    return out;
}

@group(1) @binding(0) var sprite_texture: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;

fn composite_text_layers(
    fill_color: vec4<f32>,
    outline_color: vec4<f32>,
    shadow_color: vec4<f32>,
    fill_cov: f32,
    outline_cov: f32,
    shadow_cov: f32,
) -> vec4<f32> {
    let fill_alpha = fill_color.a * fill_cov;
    let outline_alpha = outline_color.a * outline_cov;
    let shadow_alpha = shadow_color.a * shadow_cov;
    let outline_weight = outline_alpha * (1.0 - fill_alpha);
    let shadow_weight = shadow_alpha * (1.0 - fill_alpha) * (1.0 - outline_alpha);
    let alpha = fill_alpha + outline_weight + shadow_weight;

    if alpha <= 0.0 {
        return vec4<f32>(0.0);
    }

    let rgb = (
        fill_color.rgb * fill_alpha
        + outline_color.rgb * outline_weight
        + shadow_color.rgb * shadow_weight
    ) / alpha;
    return vec4<f32>(rgb, alpha);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = vec4<f32>(0.0);
    let base_sample = textureSample(sprite_texture, sprite_sampler, in.uv);

    if (in.effect_flags & TEXT_EFFECT_TEXT) == 0u {
        color = in.color * base_sample;
    } else {
        let fill_cov = base_sample.a;
        let outline_cov = select(0.0, base_sample.r, (in.effect_flags & TEXT_EFFECT_OUTLINE) != 0u);
        var shadow_cov = 0.0;
        if (in.effect_flags & TEXT_EFFECT_SHADOW) != 0u {
            let shadow_uv = in.uv - in.effect_params.xy;
            shadow_cov = textureSampleLevel(sprite_texture, sprite_sampler, shadow_uv, 0.0).a
                * (1.0 - max(fill_cov, outline_cov));
        }

        color = composite_text_layers(
            in.color,
            in.outline_color,
            in.shadow_color,
            fill_cov,
            outline_cov,
            shadow_cov,
        );
    }

#ifdef TONEMAP_IN_SHADER
    color = tonemapping::tone_mapping(color, view.color_grading);
#endif

    return color;
}

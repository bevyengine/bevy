#ifdef TONEMAP_IN_SHADER
#import bevy_core_pipeline::tonemapping
#endif

#import bevy_render::{
    maths::affine3_to_square,
    view::View,
}

#import bevy_sprite::sprite_view_bindings::view

const TEXT_EFFECT_NONE: u32 = 0u;
const TEXT_EFFECT_SHADOW: u32 = 1u;

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
    @location(7) i_effect_flags: vec4<u32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) color: vec4<f32>,
    @location(2) @interpolate(flat) shadow_color: vec4<f32>,
    @location(3) @interpolate(flat) effect_params: vec4<f32>,
    @location(4) @interpolate(flat) effect_kind: u32,
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
    out.effect_params = in.i_effect_params;
    out.effect_kind = in.i_effect_flags.x;

    return out;
}

@group(1) @binding(0) var sprite_texture: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;

fn composite_text_layers(
    fill_color: vec4<f32>,
    shadow_color: vec4<f32>,
    fill_cov: f32,
    shadow_cov: f32,
) -> vec4<f32> {
    let fill_alpha = fill_color.a * fill_cov;
    let shadow_alpha = shadow_color.a * shadow_cov;
    let shadow_weight = shadow_alpha * (1.0 - fill_alpha);
    let alpha = fill_alpha + shadow_weight;

    if alpha <= 0.0 {
        return vec4<f32>(0.0);
    }

    let rgb = (fill_color.rgb * fill_alpha + shadow_color.rgb * shadow_weight) / alpha;
    return vec4<f32>(rgb, alpha);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = vec4<f32>(0.0);
    let base_sample = textureSample(sprite_texture, sprite_sampler, in.uv);

    if in.effect_kind == TEXT_EFFECT_NONE {
        color = in.color * base_sample;
    } else {
        let fill_cov = base_sample.a;
        if in.effect_kind == TEXT_EFFECT_SHADOW {
            let shadow_uv = in.uv - in.effect_params.xy;
            let shadow_cov = textureSampleLevel(sprite_texture, sprite_sampler, shadow_uv, 0.0).a;
            color = composite_text_layers(
                in.color,
                in.shadow_color,
                fill_cov,
                shadow_cov * (1.0 - fill_cov),
            );
        }
    }

#ifdef TONEMAP_IN_SHADER
    color = tonemapping::tone_mapping(color, view.color_grading);
#endif

    return color;
}

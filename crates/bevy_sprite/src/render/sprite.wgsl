#ifdef TONEMAP_IN_SHADER
#import bevy_core_pipeline::tonemapping
#endif

#import bevy_render::{
    maths::affine_to_square,
    view::View,
}

@group(0) @binding(0) var<uniform> view: View;

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

#ifdef MASK
    @location(5) i_mask_model_transpose_col0: vec4<f32>,
    @location(6) i_mask_model_transpose_col1: vec4<f32>,
    @location(7) i_mask_model_transpose_col2: vec4<f32>,
    @location(8) i_mask_uv_offset_scale: vec4<f32>,
#endif
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) color: vec4<f32>,

#ifdef MASK
    @location(2) mask_uv: vec2<f32>,
#endif
};

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let vertex_position = vec3<f32>(
        f32(in.index & 0x1u),
        f32((in.index & 0x2u) >> 1u),
        0.0
    );

    out.clip_position = view.view_proj * affine_to_square(mat3x4<f32>(
        in.i_model_transpose_col0,
        in.i_model_transpose_col1,
        in.i_model_transpose_col2,
    )) * vec4<f32>(vertex_position, 1.0);
    out.uv = vec2<f32>(vertex_position.xy) * in.i_uv_offset_scale.zw + in.i_uv_offset_scale.xy;

#ifdef MASK
    let mask_position = 
        affine_to_square(mat3x4<f32>(
            in.i_mask_model_transpose_col0,
            in.i_mask_model_transpose_col1,
            in.i_mask_model_transpose_col2,
        )) * vec4<f32>(vertex_position, 1.0);
    out.mask_uv = vec2<f32>(mask_position.xy) * in.i_mask_uv_offset_scale.zw + in.i_mask_uv_offset_scale.xy;
#endif

    out.color = in.i_color;

    return out;
}

@group(1) @binding(0) var sprite_texture: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;

#ifdef MASK

@group(2) @binding(0) var mask_texture: texture_2d<f32>;
@group(2) @binding(1) var mask_sampler: sampler;

#ifdef MASK_THRESHOLD

struct SpriteMaskUniform {
    threshold: f32,

    #ifdef SIXTEEN_BYTE_ALIGNMENT
        // WebGL2 structs must be 16 byte aligned.
        _padding_a: f32,
        _padding_b: f32,
        _padding_c: f32,
    #endif
}

@group(3) @binding(0) var<uniform> mask_uniform: SpriteMaskUniform;
#endif

#endif

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = in.color * textureSample(sprite_texture, sprite_sampler, in.uv);

#ifdef TONEMAP_IN_SHADER
    color = tonemapping::tone_mapping(color, view.color_grading);
#endif

#ifdef MASK
    var mask = textureSample(mask_texture, mask_sampler, in.mask_uv).x;

#ifdef MASK_THRESHOLD
    mask = step(mask_uniform.threshold, mask);
#endif

    color.a *= mask;
#endif

    return color;
}

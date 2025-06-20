#ifdef TONEMAP_IN_SHADER
#import bevy_core_pipeline::tonemapping
#endif

#import bevy_render::{
    maths::affine3_to_square,
    view::View,
}

#import bevy_sprite::sprite_view_bindings::view

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
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) color: vec4<f32>,
    @location(2) world_pos: vec2<f32>,
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

    let model = mat3x4<f32>(
        in.i_model_transpose_col0,
        in.i_model_transpose_col1,
        in.i_model_transpose_col2,
    );
    let world_pos_3d = transpose(model) * vec4<f32>(vertex_position, 1.0);
    out.world_pos = world_pos_3d.xy;

    return out;
}

@group(1) @binding(0) var sprite_texture: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;

struct GpuPointLight2D {
    color_intensity: vec4<f32>,
    position_radius: vec4<f32>, 
};

@group(2) @binding(0)
var<uniform> point_lights: array<GpuPointLight2D, 16>;

fn compute_lighting(pos: vec2<f32>) -> vec3<f32> {
    var lighting: vec3<f32> = vec3(0.0);

    for (var i = 0u; i < 16u; i = i + 1u) {
        let light = point_lights[i];

        let dist = distance(pos, light.position_radius.xy);
        if (dist > light.position_radius.z) {
            continue;
        }

        let attenuation = select(
            1.0 - dist / light.position_radius.z,
            pow(1.0 - dist / light.position_radius.z, 2.0),
            light.position_radius.w == 2.0
        );

        lighting += light.color_intensity.rgb * light.color_intensity.a * attenuation;
    }

    return lighting;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = in.color * textureSample(sprite_texture, sprite_sampler, in.uv);
#ifdef TONEMAP_IN_SHADER
    color = tonemapping::tone_mapping(color, view.color_grading);
#endif

    let lighting = compute_lighting(in.world_pos);
    return vec4(color.rgb * lighting, color.a);
}

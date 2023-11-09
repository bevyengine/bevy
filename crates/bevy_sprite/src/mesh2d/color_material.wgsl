#import bevy_sprite::{
    mesh2d_vertex_output::VertexOutput,
    mesh2d_view_bindings::view,
}

#ifdef TONEMAP_IN_SHADER
#import bevy_core_pipeline::tonemapping
#endif

struct ColorMaterial {
    color: vec4<f32>,
    // 'flags' is a bit field indicating various options. u32 is 32 bits so we have up to 32 options.
    flags: u32,
};
const COLOR_MATERIAL_FLAGS_TEXTURE_BIT: u32 = 1u;

@group(2) @binding(0) var<uniform> material: ColorMaterial;
@group(2) @binding(1) var texture: texture_2d<f32>;
@group(2) @binding(2) var texture_sampler: sampler;

@fragment
fn fragment(
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
    var output_color: vec4<f32> = material.color;
#ifdef VERTEX_COLORS
    output_color = output_color * mesh.color;
#endif
    if ((material.flags & COLOR_MATERIAL_FLAGS_TEXTURE_BIT) != 0u) {
        output_color = output_color * textureSample(texture, texture_sampler, mesh.uv);
    }
#ifdef TONEMAP_IN_SHADER
    output_color = tonemapping::tone_mapping(output_color, view.color_grading);
#endif
    return output_color;
}

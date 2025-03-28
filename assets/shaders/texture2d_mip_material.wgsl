#import bevy_sprite::{
    mesh2d_vertex_output::VertexOutput,
    mesh2d_view_bindings::view,
}

#ifdef TONEMAP_IN_SHADER
#import bevy_core_pipeline::tonemapping
#endif

struct TextureMipMaterial {
    texture_available: u32,
    mip_level: f32,
    layer_index: u32,
}

@group(2) @binding(0) var<uniform> material: TextureMipMaterial;
@group(2) @binding(1) var texture: texture_2d<f32>;
@group(2) @binding(2) var texture_sampler: sampler;

@fragment
fn fragment(
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
    var output_color: vec4<f32> = vec4<f32>(0.0);
    if (material.texture_available != 0u) {
        output_color = textureSampleLevel(texture, texture_sampler, mesh.uv, material.mip_level);
    }

#ifdef TONEMAP_IN_SHADER
    output_color = tonemapping::tone_mapping(output_color, view.color_grading);
#endif
    return output_color;
}

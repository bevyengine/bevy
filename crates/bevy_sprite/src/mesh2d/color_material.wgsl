#import bevy_sprite::mesh2d_types
#import bevy_sprite::mesh2d_vertex_output

struct ColorMaterial {
    color: vec4<f32>,
    // 'flags' is a bit field indicating various options. u32 is 32 bits so we have up to 32 options.
    flags: u32,
};
let COLOR_MATERIAL_FLAGS_TEXTURE_BIT: u32 = 1u;

@group(1) @binding(0)
var<uniform> material: ColorMaterial;
@group(1) @binding(1)
var texture: texture_2d<f32>;
@group(1) @binding(2)
var texture_sampler: sampler;

@group(2) @binding(0)
var<uniform> mesh: bevy_sprite::mesh2d_types::Mesh2d;

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: bevy_sprite::mesh2d_vertex_output::MeshVertexOutput,
) -> @location(0) vec4<f32> {
    var output_color: vec4<f32> = material.color;
    if ((material.flags & COLOR_MATERIAL_FLAGS_TEXTURE_BIT) != 0u) {
#ifdef VERTEX_COLORS
        output_color = output_color * textureSample(texture, texture_sampler, mesh.uv) * mesh.color;
#else
        output_color = output_color * textureSample(texture, texture_sampler, mesh.uv);
#endif
    }
    return output_color;
}

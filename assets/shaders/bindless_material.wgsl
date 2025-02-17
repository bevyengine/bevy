#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_bindings::mesh

struct Color {
    base_color: vec4<f32>,
}

#ifdef BINDLESS
@group(2) @binding(0) var<storage> material_color: binding_array<Color, 4>;
@group(2) @binding(1) var material_color_texture: binding_array<texture_2d<f32>, 4>;
@group(2) @binding(2) var material_color_sampler: binding_array<sampler, 4>;
#else   // BINDLESS
@group(2) @binding(0) var<uniform> material_color: Color;
@group(2) @binding(1) var material_color_texture: texture_2d<f32>;
@group(2) @binding(2) var material_color_sampler: sampler;
#endif  // BINDLESS

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
#ifdef BINDLESS
    let slot = mesh[in.instance_index].material_and_lightmap_bind_group_slot & 0xffffu;
    let base_color = material_color[slot].base_color;
#else   // BINDLESS
    let base_color = material_color.base_color;
#endif  // BINDLESS

    return base_color * textureSampleLevel(
#ifdef BINDLESS
        material_color_texture[slot],
        material_color_sampler[slot],
#else   // BINDLESS
        material_color_texture,
        material_color_sampler,
#endif  // BINDLESS
        in.uv,
        0.0
    );
}

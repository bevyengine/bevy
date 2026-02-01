#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_bindings::mesh
#import bevy_render::bindless::{bindless_samplers_filtering, bindless_textures_2d}

struct Color {
    base_color: vec4<f32>,
}

// This structure is a mapping from bindless index to the index in the
// appropriate slab
struct MaterialBindings {
    material: u32,              // 0
    color_texture: u32,         // 1
    color_texture_sampler: u32, // 2
}

#ifdef BINDLESS
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<storage> materials: array<MaterialBindings>;
@group(#{MATERIAL_BIND_GROUP}) @binding(10) var<storage> material_color: binding_array<Color>;
#else   // BINDLESS
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> material_color: Color;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var material_color_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var material_color_sampler: sampler;
#endif  // BINDLESS

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
#ifdef BINDLESS
    let slot = mesh[in.instance_index].material_and_lightmap_bind_group_slot & 0xffffu;
    let base_color = material_color[materials[slot].material].base_color;
#else   // BINDLESS
    let base_color = material_color.base_color;
#endif  // BINDLESS

    return base_color * textureSampleLevel(
#ifdef BINDLESS
        bindless_textures_2d[materials[slot].color_texture],
        bindless_samplers_filtering[materials[slot].color_texture_sampler],
#else   // BINDLESS
        material_color_texture,
        material_color_sampler,
#endif  // BINDLESS
        in.uv,
        0.0
    );
}

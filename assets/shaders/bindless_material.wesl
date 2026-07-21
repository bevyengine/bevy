import bevy_pbr::render::forward_io::VertexOutput;
import bevy_pbr::render::mesh_bindings::mesh;
import bevy_render::bindless::{bindless_samplers_filtering, bindless_textures_2d};

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

@if(BINDLESS)
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<storage> materials: array<MaterialBindings>;
@if(BINDLESS)
@group(constants::MATERIAL_BIND_GROUP) @binding(10) var<storage> material_color: binding_array<Color>;
@if(!BINDLESS)
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> material_color: Color;
@if(!BINDLESS)
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var material_color_texture: texture_2d<f32>;
@if(!BINDLESS)
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var material_color_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    @if(BINDLESS)
    let slot = mesh[in.instance_index].material_and_lightmap_bind_group_slot & 0xffffu;
    @if(BINDLESS)
    let base_color = material_color[materials[slot].material].base_color;
    @else   // BINDLESS
    let base_color = material_color.base_color;

    @if(BINDLESS)
    let texture = bindless_textures_2d[materials[slot].color_texture];
    @else
    let texture = material_color_texture;
    @if(BINDLESS)
    let color_sampler = bindless_samplers_filtering[materials[slot].color_texture_sampler];
    @else
    let color_sampler = material_color_sampler;
    return base_color * textureSampleLevel(
        texture,
        color_sampler,
        in.uv,
        0.0
    );
}

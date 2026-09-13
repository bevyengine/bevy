import bevy_sprite_render::mesh2d::vertex_output::VertexOutput;
import bevy_sprite_render::mesh2d::bindings::mesh;
import bevy_render::bindless::{bindless_samplers_filtering, bindless_textures_2d};
// we can import items from shader modules in the assets folder with a relative path
import super::custom_material_import::COLOR_MULTIPLIER;

public struct Color {
    base_color: vec4<f32>,
}

/// This structure is a mapping from bindless index to the index in the
/// appropriate slab
public struct MaterialBindings {
    material: u32,              // 0
    color_texture: u32,         // 1
    color_texture_sampler: u32, // 2
}

@if(BINDLESS) {
@group(constants::MATERIAL_BIND_GROUP) @binding(0) public var<storage> materials: array<MaterialBindings>;
@group(constants::MATERIAL_BIND_GROUP) @binding(10) public var<storage> material_color: binding_array<Color>;
}
@else {
@group(constants::MATERIAL_BIND_GROUP) @binding(0) public var<uniform> material_color: Color;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) public var material_color_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) public var material_color_sampler: sampler;
}

@fragment
public fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    @if(BINDLESS) {
    let slot = mesh[in.instance_index].material_bind_group_slot;
    let base_color = material_color[materials[slot].material].base_color;
    let texture = bindless_textures_2d[materials[slot].color_texture];
    let color_sampler = bindless_samplers_filtering[materials[slot].color_texture_sampler];
    }
    @else {
    let base_color = material_color.base_color;
    let texture = material_color_texture;
    let color_sampler = material_color_sampler;
    }
    return base_color * textureSampleLevel(
        texture,
        color_sampler,
        in.uv,
        0.0
    );
}

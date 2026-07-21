import bevy_sprite_render::sprite_mesh::types::SpriteMaterial;
@if(BINDLESS)
import bevy_sprite_render::sprite_mesh::types::SpriteMaterialBindings;

@if(BINDLESS)
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<storage> material_indices:
    array<SpriteMaterialBindings>;
@if(BINDLESS)
@group(constants::MATERIAL_BIND_GROUP) @binding(10) var<storage> material: array<SpriteMaterial>;

@if(!BINDLESS)
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> material: SpriteMaterial;
@if(!BINDLESS)
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var texture: texture_2d<f32>;
@if(!BINDLESS)
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var texture_sampler: sampler;

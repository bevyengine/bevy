import package::sprite_mesh::types::SpriteMaterial;
@if(BINDLESS)
import package::sprite_mesh::types::SpriteMaterialBindings;

@if(BINDLESS) {
@group(constants::MATERIAL_BIND_GROUP) @binding(0) public var<storage> material_indices:
    array<SpriteMaterialBindings>;
@group(constants::MATERIAL_BIND_GROUP) @binding(10) public var<storage> material: array<SpriteMaterial>;
}
@else {
@group(constants::MATERIAL_BIND_GROUP) @binding(0) public var<uniform> material: SpriteMaterial;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) public var texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) public var texture_sampler: sampler;
}

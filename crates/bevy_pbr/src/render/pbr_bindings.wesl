import package::render::pbr_types::StandardMaterial;

@if(BINDLESS) {
public struct StandardMaterialBindings {
    material: u32,                      // 0
    base_color_texture: u32,            // 1
    base_color_sampler: u32,            // 2
    emissive_texture: u32,              // 3
    emissive_sampler: u32,              // 4
    metallic_roughness_texture: u32,    // 5
    metallic_roughness_sampler: u32,    // 6
    occlusion_texture: u32,             // 7
    occlusion_sampler: u32,             // 8
    normal_map_texture: u32,            // 9
    normal_map_sampler: u32,            // 10
    depth_map_texture: u32,             // 11
    depth_map_sampler: u32,             // 12
    anisotropy_texture: u32,            // 13
    anisotropy_sampler: u32,            // 14
    specular_transmission_texture: u32, // 15
    specular_transmission_sampler: u32, // 16
    thickness_texture: u32,             // 17
    thickness_sampler: u32,             // 18
    diffuse_transmission_texture: u32,  // 19
    diffuse_transmission_sampler: u32,  // 20
    clearcoat_texture: u32,             // 21
    clearcoat_sampler: u32,             // 22
    clearcoat_roughness_texture: u32,   // 23
    clearcoat_roughness_sampler: u32,   // 24
    clearcoat_normal_texture: u32,      // 25
    clearcoat_normal_sampler: u32,      // 26
    specular_texture: u32,              // 27
    specular_sampler: u32,              // 28
    specular_tint_texture: u32,         // 29
    specular_tint_sampler: u32,         // 30
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) public var<storage> material_indices: array<StandardMaterialBindings>;
@group(constants::MATERIAL_BIND_GROUP) @binding(10) public var<storage> material_array: array<StandardMaterial>;
}
@else {
@group(constants::MATERIAL_BIND_GROUP) @binding(0) public var<uniform> material: StandardMaterial;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) public var base_color_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) public var base_color_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) public var emissive_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(4) public var emissive_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(5) public var metallic_roughness_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(6) public var metallic_roughness_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(7) public var occlusion_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(8) public var occlusion_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(9) public var normal_map_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(10) public var normal_map_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(11) public var depth_map_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(12) public var depth_map_sampler: sampler;
    @if(PBR_ANISOTROPY_TEXTURE_SUPPORTED) {
    @group(constants::MATERIAL_BIND_GROUP) @binding(13) public var anisotropy_texture: texture_2d<f32>;
    @group(constants::MATERIAL_BIND_GROUP) @binding(14) public var anisotropy_sampler: sampler;
    }

    @if(PBR_TRANSMISSION_TEXTURES_SUPPORTED) {
    @group(constants::MATERIAL_BIND_GROUP) @binding(15) public var specular_transmission_texture: texture_2d<f32>;
    @group(constants::MATERIAL_BIND_GROUP) @binding(16) public var specular_transmission_sampler: sampler;
    @group(constants::MATERIAL_BIND_GROUP) @binding(17) public var thickness_texture: texture_2d<f32>;
    @group(constants::MATERIAL_BIND_GROUP) @binding(18) public var thickness_sampler: sampler;
    @group(constants::MATERIAL_BIND_GROUP) @binding(19) public var diffuse_transmission_texture: texture_2d<f32>;
    @group(constants::MATERIAL_BIND_GROUP) @binding(20) public var diffuse_transmission_sampler: sampler;
    }

    @if(PBR_MULTI_LAYER_MATERIAL_TEXTURES_SUPPORTED) {
    @group(constants::MATERIAL_BIND_GROUP) @binding(21) public var clearcoat_texture: texture_2d<f32>;
    @group(constants::MATERIAL_BIND_GROUP) @binding(22) public var clearcoat_sampler: sampler;
    @group(constants::MATERIAL_BIND_GROUP) @binding(23) public var clearcoat_roughness_texture: texture_2d<f32>;
    @group(constants::MATERIAL_BIND_GROUP) @binding(24) public var clearcoat_roughness_sampler: sampler;
    @group(constants::MATERIAL_BIND_GROUP) @binding(25) public var clearcoat_normal_texture: texture_2d<f32>;
    @group(constants::MATERIAL_BIND_GROUP) @binding(26) public var clearcoat_normal_sampler: sampler;
    }

    @if(PBR_SPECULAR_TEXTURES_SUPPORTED) {
    @group(constants::MATERIAL_BIND_GROUP) @binding(27) public var specular_texture: texture_2d<f32>;
    @group(constants::MATERIAL_BIND_GROUP) @binding(28) public var specular_sampler: sampler;
    @group(constants::MATERIAL_BIND_GROUP) @binding(29) public var specular_tint_texture: texture_2d<f32>;
    @group(constants::MATERIAL_BIND_GROUP) @binding(30) public var specular_tint_sampler: sampler;
    }
}

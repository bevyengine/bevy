#define_import_path bevy_pbr::pbr_bindings

#import bevy_pbr::pbr_types::StandardMaterial

#ifdef BINDLESS
struct StandardMaterialBindings {
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

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<storage> material_indices: array<StandardMaterialBindings>;
@group(#{MATERIAL_BIND_GROUP}) @binding(10) var<storage> material_array: array<StandardMaterial>;

#else   // BINDLESS

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> material: StandardMaterial;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var base_color_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var base_color_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var emissive_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var emissive_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(5) var metallic_roughness_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(6) var metallic_roughness_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(7) var occlusion_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(8) var occlusion_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(9) var normal_map_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(10) var normal_map_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(11) var depth_map_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(12) var depth_map_sampler: sampler;

#ifdef PBR_ANISOTROPY_TEXTURE_SUPPORTED
@group(#{MATERIAL_BIND_GROUP}) @binding(13) var anisotropy_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(14) var anisotropy_sampler: sampler;
#endif  // PBR_ANISOTROPY_TEXTURE_SUPPORTED

#ifdef PBR_TRANSMISSION_TEXTURES_SUPPORTED
@group(#{MATERIAL_BIND_GROUP}) @binding(15) var specular_transmission_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(16) var specular_transmission_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(17) var thickness_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(18) var thickness_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(19) var diffuse_transmission_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(20) var diffuse_transmission_sampler: sampler;
#endif  // PBR_TRANSMISSION_TEXTURES_SUPPORTED

#ifdef PBR_MULTI_LAYER_MATERIAL_TEXTURES_SUPPORTED
@group(#{MATERIAL_BIND_GROUP}) @binding(21) var clearcoat_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(22) var clearcoat_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(23) var clearcoat_roughness_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(24) var clearcoat_roughness_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(25) var clearcoat_normal_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(26) var clearcoat_normal_sampler: sampler;
#endif  // PBR_MULTI_LAYER_MATERIAL_TEXTURES_SUPPORTED

#ifdef PBR_SPECULAR_TEXTURES_SUPPORTED
@group(#{MATERIAL_BIND_GROUP}) @binding(27) var specular_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(28) var specular_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(29) var specular_tint_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(30) var specular_tint_sampler: sampler;
#endif  // PBR_SPECULAR_TEXTURES_SUPPORTED

#endif  // BINDLESS

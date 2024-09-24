#define_import_path bevy_pbr::pbr_bindings

#import bevy_pbr::pbr_types::StandardMaterial

@group(2) @binding(0) var<uniform> material: StandardMaterial;
@group(2) @binding(1) var base_color_texture: texture_2d<f32>;
@group(2) @binding(2) var base_color_sampler: sampler;
@group(2) @binding(3) var emissive_texture: texture_2d<f32>;
@group(2) @binding(4) var emissive_sampler: sampler;
@group(2) @binding(5) var metallic_roughness_texture: texture_2d<f32>;
@group(2) @binding(6) var metallic_roughness_sampler: sampler;
@group(2) @binding(7) var occlusion_texture: texture_2d<f32>;
@group(2) @binding(8) var occlusion_sampler: sampler;
@group(2) @binding(9) var normal_map_texture: texture_2d<f32>;
@group(2) @binding(10) var normal_map_sampler: sampler;
@group(2) @binding(11) var depth_map_texture: texture_2d<f32>;
@group(2) @binding(12) var depth_map_sampler: sampler;
#ifdef PBR_ANISOTROPY_TEXTURE_SUPPORTED
@group(2) @binding(13) var anisotropy_texture: texture_2d<f32>;
@group(2) @binding(14) var anisotropy_sampler: sampler;
#endif
#ifdef PBR_TRANSMISSION_TEXTURES_SUPPORTED
@group(2) @binding(15) var specular_transmission_texture: texture_2d<f32>;
@group(2) @binding(16) var specular_transmission_sampler: sampler;
@group(2) @binding(17) var thickness_texture: texture_2d<f32>;
@group(2) @binding(18) var thickness_sampler: sampler;
@group(2) @binding(19) var diffuse_transmission_texture: texture_2d<f32>;
@group(2) @binding(20) var diffuse_transmission_sampler: sampler;
#endif
#ifdef PBR_MULTI_LAYER_MATERIAL_TEXTURES_SUPPORTED
@group(2) @binding(21) var clearcoat_texture: texture_2d<f32>;
@group(2) @binding(22) var clearcoat_sampler: sampler;
@group(2) @binding(23) var clearcoat_roughness_texture: texture_2d<f32>;
@group(2) @binding(24) var clearcoat_roughness_sampler: sampler;
@group(2) @binding(25) var clearcoat_normal_texture: texture_2d<f32>;
@group(2) @binding(26) var clearcoat_normal_sampler: sampler;
#endif

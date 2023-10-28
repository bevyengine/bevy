#define_import_path bevy_pbr::pbr_bindings

#import bevy_pbr::pbr_types::StandardMaterial

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

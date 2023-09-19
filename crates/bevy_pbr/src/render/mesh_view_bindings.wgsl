#define_import_path bevy_pbr::mesh_view_bindings

#import bevy_pbr::mesh_view_types as types
#import bevy_render::view  View
#import bevy_render::globals  Globals

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<uniform> lights: types::Lights;
#ifdef NO_ARRAY_TEXTURES_SUPPORT
@group(0) @binding(2) var point_shadow_textures: texture_depth_cube;
#else
@group(0) @binding(2) var point_shadow_textures: texture_depth_cube_array;
#endif
@group(0) @binding(3) var point_shadow_textures_sampler: sampler_comparison;
#ifdef NO_ARRAY_TEXTURES_SUPPORT
@group(0) @binding(4) var directional_shadow_textures: texture_depth_2d;
#else
@group(0) @binding(4) var directional_shadow_textures: texture_depth_2d_array;
#endif
@group(0) @binding(5) var directional_shadow_textures_sampler: sampler_comparison;

#if AVAILABLE_STORAGE_BUFFER_BINDINGS >= 3
@group(0) @binding(6) var<storage> point_lights: types::PointLights;
@group(0) @binding(7) var<storage> cluster_light_index_lists: types::ClusterLightIndexLists;
@group(0) @binding(8) var<storage> cluster_offsets_and_counts: types::ClusterOffsetsAndCounts;
#else
@group(0) @binding(6) var<uniform> point_lights: types::PointLights;
@group(0) @binding(7) var<uniform> cluster_light_index_lists: types::ClusterLightIndexLists;
@group(0) @binding(8) var<uniform> cluster_offsets_and_counts: types::ClusterOffsetsAndCounts;
#endif

@group(0) @binding(9) var<uniform> globals: Globals;
@group(0) @binding(10) var<uniform> fog: types::Fog;

@group(0) @binding(11) var screen_space_ambient_occlusion_texture: texture_2d<f32>;

@group(0) @binding(12) var environment_map_diffuse: texture_cube<f32>;
@group(0) @binding(13) var environment_map_specular: texture_cube<f32>;
@group(0) @binding(14) var environment_map_sampler: sampler;

@group(0) @binding(15) var dt_lut_texture: texture_3d<f32>;
@group(0) @binding(16) var dt_lut_sampler: sampler;

#ifdef MULTISAMPLED
@group(0) @binding(17) var depth_prepass_texture: texture_depth_multisampled_2d;
@group(0) @binding(18) var normal_prepass_texture: texture_multisampled_2d<f32>;
@group(0) @binding(19) var motion_vector_prepass_texture: texture_multisampled_2d<f32>;
#else
@group(0) @binding(17) var depth_prepass_texture: texture_depth_2d;
@group(0) @binding(18) var normal_prepass_texture: texture_2d<f32>;
@group(0) @binding(19) var motion_vector_prepass_texture: texture_2d<f32>;
#endif

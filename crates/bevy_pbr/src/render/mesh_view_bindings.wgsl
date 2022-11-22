#define_import_path bevy_pbr::mesh_view_bindings

#import bevy_pbr::mesh_view_types as types
#import bevy_render::core_bindings

@group(0) @binding(auto)
var<uniform> lights: types::Lights;
#ifdef NO_ARRAY_TEXTURES_SUPPORT
@group(0) @binding(auto)
var point_shadow_textures: texture_depth_cube;
#else
@group(0) @binding(auto)
var point_shadow_textures: texture_depth_cube_array;
#endif
@group(0) @binding(auto)
var point_shadow_textures_sampler: sampler_comparison;
#ifdef NO_ARRAY_TEXTURES_SUPPORT
@group(0) @binding(auto)
var directional_shadow_textures: texture_depth_2d;
#else
@group(0) @binding(auto)
var directional_shadow_textures: texture_depth_2d_array;
#endif
@group(0) @binding(auto)
var directional_shadow_textures_sampler: sampler_comparison;

#ifdef NO_STORAGE_BUFFERS_SUPPORT
@group(0) @binding(auto)
var<uniform> point_lights: types::PointLights;
@group(0) @binding(auto)
var<uniform> cluster_light_index_lists: types::ClusterLightIndexLists;
@group(0) @binding(auto)
var<uniform> cluster_offsets_and_counts: types::ClusterOffsetsAndCounts;
#else
@group(0) @binding(auto)
var<storage> point_lights: types::PointLights;
@group(0) @binding(auto)
var<storage> cluster_light_index_lists: types::ClusterLightIndexLists;
@group(0) @binding(auto)
var<storage> cluster_offsets_and_counts: types::ClusterOffsetsAndCounts;
#endif
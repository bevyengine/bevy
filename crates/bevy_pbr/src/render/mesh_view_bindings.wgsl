#define_import_path bevy_pbr::mesh_view_bindings

#import bevy_pbr::mesh_view_types

@group(0) @binding(0)
var<uniform> view: View;
@group(0) @binding(1)
var<uniform> lights: Lights;
#ifdef NO_ARRAY_TEXTURES_SUPPORT
@group(0) @binding(2)
var point_shadow_textures: texture_depth_cube;
#else
@group(0) @binding(2)
var point_shadow_textures: texture_depth_cube_array;
#endif
@group(0) @binding(3)
var point_shadow_textures_sampler: sampler_comparison;
#ifdef NO_ARRAY_TEXTURES_SUPPORT
@group(0) @binding(4)
var directional_shadow_textures: texture_depth_2d;
#else
@group(0) @binding(4)
var directional_shadow_textures: texture_depth_2d_array;
#endif
@group(0) @binding(5)
var directional_shadow_textures_sampler: sampler_comparison;

#if AVAILABLE_STORAGE_BUFFER_BINDINGS >= 3
@group(0) @binding(6)
var<storage> point_lights: PointLights;
@group(0) @binding(7)
var<storage> cluster_light_index_lists: ClusterLightIndexLists;
@group(0) @binding(8)
var<storage> cluster_offsets_and_counts: ClusterOffsetsAndCounts;
#else
@group(0) @binding(6)
var<uniform> point_lights: PointLights;
@group(0) @binding(7)
var<uniform> cluster_light_index_lists: ClusterLightIndexLists;
@group(0) @binding(8)
var<uniform> cluster_offsets_and_counts: ClusterOffsetsAndCounts;
#endif

@group(0) @binding(9)
var<uniform> globals: Globals;

#define_import_path bevy_pbr::mesh_view_bindings

#import bevy_pbr::mesh_view_types as types
#import bevy_render::{
    view::View,
    globals::Globals,
}

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<uniform> lights: types::Lights;
#ifdef NO_CUBE_ARRAY_TEXTURES_SUPPORT
@group(0) @binding(2) var point_shadow_textures: texture_depth_cube;
#else
@group(0) @binding(2) var point_shadow_textures: texture_depth_cube_array;
#endif
@group(0) @binding(3) var point_shadow_textures_comparison_sampler: sampler_comparison;
#ifdef PCSS_SAMPLERS_AVAILABLE
@group(0) @binding(4) var point_shadow_textures_linear_sampler: sampler;
#endif  // PCSS_SAMPLERS_AVAILABLE
#ifdef NO_ARRAY_TEXTURES_SUPPORT
@group(0) @binding(5) var directional_shadow_textures: texture_depth_2d;
#else
@group(0) @binding(5) var directional_shadow_textures: texture_depth_2d_array;
#endif
@group(0) @binding(6) var directional_shadow_textures_comparison_sampler: sampler_comparison;
#ifdef PCSS_SAMPLERS_AVAILABLE
@group(0) @binding(7) var directional_shadow_textures_linear_sampler: sampler;
#endif  // PCSS_SAMPLERS_AVAILABLE

#if AVAILABLE_STORAGE_BUFFER_BINDINGS >= 3
@group(0) @binding(8) var<storage> clusterable_objects: types::ClusterableObjects;
@group(0) @binding(9) var<storage> clusterable_object_index_lists: types::ClusterLightIndexLists;
@group(0) @binding(10) var<storage> cluster_offsets_and_counts: types::ClusterOffsetsAndCounts;
#else
@group(0) @binding(8) var<uniform> clusterable_objects: types::ClusterableObjects;
@group(0) @binding(9) var<uniform> clusterable_object_index_lists: types::ClusterLightIndexLists;
@group(0) @binding(10) var<uniform> cluster_offsets_and_counts: types::ClusterOffsetsAndCounts;
#endif

@group(0) @binding(11) var<uniform> globals: Globals;
@group(0) @binding(12) var<uniform> fog: types::Fog;
@group(0) @binding(13) var<uniform> light_probes: types::LightProbes;

const VISIBILITY_RANGE_UNIFORM_BUFFER_SIZE: u32 = 64u;
#if AVAILABLE_STORAGE_BUFFER_BINDINGS >= 6
@group(0) @binding(14) var<storage> visibility_ranges: array<vec4<f32>>;
#else
@group(0) @binding(14) var<uniform> visibility_ranges: array<vec4<f32>, VISIBILITY_RANGE_UNIFORM_BUFFER_SIZE>;
#endif

@group(0) @binding(15) var<uniform> ssr_settings: types::ScreenSpaceReflectionsSettings;
@group(0) @binding(16) var screen_space_ambient_occlusion_texture: texture_2d<f32>;
@group(0) @binding(17) var<uniform> environment_map_uniform: types::EnvironmentMapUniform;

// NB: If you change these, make sure to update `tonemapping_shared.wgsl` too.
@group(0) @binding(18) var dt_lut_texture: texture_3d<f32>;
@group(0) @binding(19) var dt_lut_sampler: sampler;

#ifdef MULTISAMPLED
#ifdef DEPTH_PREPASS
@group(0) @binding(20) var depth_prepass_texture: texture_depth_multisampled_2d;
#endif // DEPTH_PREPASS
#ifdef NORMAL_PREPASS
@group(0) @binding(21) var normal_prepass_texture: texture_multisampled_2d<f32>;
#endif // NORMAL_PREPASS
#ifdef MOTION_VECTOR_PREPASS
@group(0) @binding(22) var motion_vector_prepass_texture: texture_multisampled_2d<f32>;
#endif // MOTION_VECTOR_PREPASS

#else // MULTISAMPLED

#ifdef DEPTH_PREPASS
@group(0) @binding(20) var depth_prepass_texture: texture_depth_2d;
#endif // DEPTH_PREPASS
#ifdef NORMAL_PREPASS
@group(0) @binding(21) var normal_prepass_texture: texture_2d<f32>;
#endif // NORMAL_PREPASS
#ifdef MOTION_VECTOR_PREPASS
@group(0) @binding(22) var motion_vector_prepass_texture: texture_2d<f32>;
#endif // MOTION_VECTOR_PREPASS

#endif // MULTISAMPLED

#ifdef DEFERRED_PREPASS
@group(0) @binding(23) var deferred_prepass_texture: texture_2d<u32>;
#endif // DEFERRED_PREPASS

@group(0) @binding(24) var view_transmission_texture: texture_2d<f32>;
@group(0) @binding(25) var view_transmission_sampler: sampler;

#ifdef OIT_ENABLED
@group(0) @binding(26) var<storage, read_write> oit_layers: array<vec2<u32>>;
@group(0) @binding(27) var<storage, read_write> oit_layer_ids: array<atomic<i32>>;
@group(0) @binding(28) var<uniform> oit_settings: types::OrderIndependentTransparencySettings;
#endif // OIT_ENABLED

#ifdef MULTIPLE_LIGHT_PROBES_IN_ARRAY
@group(1) @binding(0) var diffuse_environment_maps: binding_array<texture_cube<f32>, 8u>;
@group(1) @binding(1) var specular_environment_maps: binding_array<texture_cube<f32>, 8u>;
#else
@group(1) @binding(0) var diffuse_environment_map: texture_cube<f32>;
@group(1) @binding(1) var specular_environment_map: texture_cube<f32>;
#endif
@group(1) @binding(2) var environment_map_sampler: sampler;

#ifdef IRRADIANCE_VOLUMES_ARE_USABLE
#ifdef MULTIPLE_LIGHT_PROBES_IN_ARRAY
@group(1) @binding(3) var irradiance_volumes: binding_array<texture_3d<f32>, 8u>;
#else
@group(1) @binding(3) var irradiance_volume: texture_3d<f32>;
#endif
@group(1) @binding(4) var irradiance_volume_sampler: sampler;
#endif

#ifdef CLUSTERED_DECALS_ARE_USABLE
@group(1) @binding(5) var<storage> clustered_decals: types::ClusteredDecals;
@group(1) @binding(6) var clustered_decal_textures: binding_array<texture_2d<f32>, 8u>;
@group(1) @binding(7) var clustered_decal_sampler: sampler;
#endif  // CLUSTERED_DECALS_ARE_USABLE

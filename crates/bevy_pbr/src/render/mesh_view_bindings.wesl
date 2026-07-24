import bevy_pbr::render::mesh_view_types as types;
import bevy_pbr::atmosphere::types as atmosphere_types;
import bevy_render::{
    view::View,
    globals::Globals,
};

@if(MULTIPLE_LIGHT_PROBES_IN_ARRAY || CLUSTERED_DECALS_ARE_USABLE)
enable wgpu_binding_array;

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<uniform> lights: types::Lights;
@if(NO_CUBE_ARRAY_TEXTURES_SUPPORT)
@group(0) @binding(2) var point_shadow_textures: texture_depth_cube;
@else
@group(0) @binding(2) var point_shadow_textures: texture_depth_cube_array;
@group(0) @binding(3) var point_shadow_textures_comparison_sampler: sampler_comparison;
@if(PCSS_SAMPLERS_AVAILABLE)
@group(0) @binding(4) var point_shadow_textures_linear_sampler: sampler;
@if(NO_ARRAY_TEXTURES_SUPPORT)
@group(0) @binding(5) var directional_shadow_textures: texture_depth_2d;
@else
@group(0) @binding(5) var directional_shadow_textures: texture_depth_2d_array;
@group(0) @binding(6) var directional_shadow_textures_comparison_sampler: sampler_comparison;
@if(PCSS_SAMPLERS_AVAILABLE)
@group(0) @binding(7) var directional_shadow_textures_linear_sampler: sampler;

@if(AVAILABLE_STORAGE_BUFFER_BINDINGS__GE_3)
@group(0) @binding(8) var<storage> clustered_lights: types::ClusteredLights;
@if(AVAILABLE_STORAGE_BUFFER_BINDINGS__GE_3)
@group(0) @binding(9) var<storage> clusterable_object_index_lists: types::ClusterableObjectIndexLists;
@if(AVAILABLE_STORAGE_BUFFER_BINDINGS__GE_3)
@group(0) @binding(10) var<storage> cluster_offsets_and_counts: types::ClusterOffsetsAndCounts;
@if(!AVAILABLE_STORAGE_BUFFER_BINDINGS__GE_3)
@group(0) @binding(8) var<uniform> clustered_lights: types::ClusteredLights;
@if(!AVAILABLE_STORAGE_BUFFER_BINDINGS__GE_3)
@group(0) @binding(9) var<uniform> clusterable_object_index_lists: types::ClusterableObjectIndexLists;
@if(!AVAILABLE_STORAGE_BUFFER_BINDINGS__GE_3)
@group(0) @binding(10) var<uniform> cluster_offsets_and_counts: types::ClusterOffsetsAndCounts;

@group(0) @binding(11) var<uniform> globals: Globals;
@group(0) @binding(12) var<uniform> light_probes: types::LightProbes;

@if(DISTANCE_FOG)
@group(0) @binding(13) var<uniform> fog: types::Fog;

const VISIBILITY_RANGE_UNIFORM_BUFFER_SIZE: u32 = 64u;
@if(AVAILABLE_STORAGE_BUFFER_BINDINGS__GE_6)
@group(0) @binding(14) var<storage> visibility_ranges: array<vec4<f32>>;
@else
@group(0) @binding(14) var<uniform> visibility_ranges: array<vec4<f32>, VISIBILITY_RANGE_UNIFORM_BUFFER_SIZE>;

@if(SCREEN_SPACE_REFLECTIONS)
@group(0) @binding(15) var<uniform> ssr_settings: types::ScreenSpaceReflectionsSettings;

@if(CONTACT_SHADOWS)
@group(0) @binding(16) var<uniform> contact_shadows_settings: types::ContactShadowsSettings;

@if(SCREEN_SPACE_AMBIENT_OCCLUSION)
@group(0) @binding(17) var screen_space_ambient_occlusion_texture: texture_2d<f32>;

@if(TONEMAP_IN_SHADER)
// NB: If you change these, make sure to update `tonemapping.wesl` too.
@group(0) @binding(18) var dt_lut_texture: texture_3d<f32>;
@if(TONEMAP_IN_SHADER)
@group(0) @binding(19) var dt_lut_sampler: sampler;

@if(MULTISAMPLED && DEPTH_PREPASS)
@group(0) @binding(20) var depth_prepass_texture: texture_depth_multisampled_2d;
@if(MULTISAMPLED && NORMAL_PREPASS)
@group(0) @binding(21) var normal_prepass_texture: texture_multisampled_2d<f32>;
@if(MULTISAMPLED && MOTION_VECTOR_PREPASS)
@group(0) @binding(22) var motion_vector_prepass_texture: texture_multisampled_2d<f32>;

@if(!MULTISAMPLED && DEPTH_PREPASS)
@group(0) @binding(20) var depth_prepass_texture: texture_depth_2d;
@if(!MULTISAMPLED && NORMAL_PREPASS)
@group(0) @binding(21) var normal_prepass_texture: texture_2d<f32>;
@if(!MULTISAMPLED && MOTION_VECTOR_PREPASS)
@group(0) @binding(22) var motion_vector_prepass_texture: texture_2d<f32>;

@if(DEFERRED_PREPASS)
@group(0) @binding(23) var deferred_prepass_texture: texture_2d<u32>;

@group(0) @binding(24) var view_transmission_texture: texture_2d<f32>;
@group(0) @binding(25) var view_transmission_sampler: sampler;

@if(OIT_ENABLED)
@group(0) @binding(26) var<uniform> oit_settings: types::OrderIndependentTransparencySettings;
@if(OIT_ENABLED)
@group(0) @binding(27) var<uniform> oit_nodes_capacity: u32;
@if(OIT_ENABLED)
@group(0) @binding(28) var<storage, read_write> oit_nodes: array<types::OitFragmentNode>;
@if(OIT_ENABLED)
@group(0) @binding(29) var<storage, read_write> oit_heads: array<atomic<u32>>;
@if(OIT_ENABLED)
@group(0) @binding(30) var<storage, read_write> oit_atomic_counter: atomic<u32>;

@if(ATMOSPHERE)
@group(0) @binding(31) var atmosphere_transmittance_texture: texture_2d<f32>;
@if(ATMOSPHERE)
@group(0) @binding(32) var atmosphere_transmittance_sampler: sampler;
@if(ATMOSPHERE)
@group(0) @binding(33) var<storage> atmosphere: atmosphere_types::Atmosphere;

@if(BLUE_NOISE_TEXTURE)
@group(0) @binding(34) var blue_noise_texture: texture_2d_array<f32>;

@if(AREA_LIGHT_LUTS)
@group(0) @binding(35) var area_light_luts: texture_2d_array<f32>;
@if(AREA_LIGHT_LUTS)
@group(0) @binding(36) var area_light_luts_sampler: sampler;

@if(DFG_LUT)
@group(0) @binding(37) var dfg_lut: texture_2d<f32>;
@if(DFG_LUT)
@group(0) @binding(38) var dfg_lut_sampler: sampler;

@if(ENVIRONMENT_MAP && MULTIPLE_LIGHT_PROBES_IN_ARRAY)
@group(1) @binding(0) var diffuse_environment_maps: binding_array<texture_cube<f32>, 8u>;
@if(ENVIRONMENT_MAP && MULTIPLE_LIGHT_PROBES_IN_ARRAY)
@group(1) @binding(1) var specular_environment_maps: binding_array<texture_cube<f32>, 8u>;
@if(ENVIRONMENT_MAP && !MULTIPLE_LIGHT_PROBES_IN_ARRAY)
@group(1) @binding(0) var diffuse_environment_map: texture_cube<f32>;
@if(ENVIRONMENT_MAP && !MULTIPLE_LIGHT_PROBES_IN_ARRAY)
@group(1) @binding(1) var specular_environment_map: texture_cube<f32>;
@if(ENVIRONMENT_MAP)
@group(1) @binding(2) var environment_map_sampler: sampler;

@if(IRRADIANCE_VOLUME && IRRADIANCE_VOLUMES_ARE_USABLE && MULTIPLE_LIGHT_PROBES_IN_ARRAY)
@group(1) @binding(3) var irradiance_volumes: binding_array<texture_3d<f32>, 8u>;
@if(IRRADIANCE_VOLUME && IRRADIANCE_VOLUMES_ARE_USABLE && !MULTIPLE_LIGHT_PROBES_IN_ARRAY)
@group(1) @binding(3) var irradiance_volume: texture_3d<f32>;
@if(IRRADIANCE_VOLUME && IRRADIANCE_VOLUMES_ARE_USABLE)
@group(1) @binding(4) var irradiance_volume_sampler: sampler;

@if(CLUSTERED_DECALS_ARE_USABLE)
@group(1) @binding(5) var<storage> clustered_decals: types::ClusteredDecals;
@if(CLUSTERED_DECALS_ARE_USABLE)
@group(1) @binding(6) var clustered_decal_textures: binding_array<texture_2d<f32>, 8u>;
@if(CLUSTERED_DECALS_ARE_USABLE)
@group(1) @binding(7) var clustered_decal_sampler: sampler;

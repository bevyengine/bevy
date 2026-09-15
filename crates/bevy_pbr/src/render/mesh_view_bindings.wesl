import package::render::mesh_view_types as types;
import package::atmosphere::types as atmosphere_types;
import bevy_render::{
    view::View,
    globals::Globals,
};

@if(MULTIPLE_LIGHT_PROBES_IN_ARRAY || CLUSTERED_DECALS_ARE_USABLE)
enable wgpu_binding_array;

@group(0) @binding(0) public var<uniform> view: View;
@group(0) @binding(1) public var<uniform> lights: types::Lights;
@if(NO_CUBE_ARRAY_TEXTURES_SUPPORT)
@group(0) @binding(2) public var point_shadow_textures: texture_depth_cube;
@else
@group(0) @binding(2) public var point_shadow_textures: texture_depth_cube_array;
@group(0) @binding(3) public var point_shadow_textures_comparison_sampler: sampler_comparison;
@if(PCSS_SAMPLERS_AVAILABLE)
@group(0) @binding(4) public var point_shadow_textures_linear_sampler: sampler;
@if(NO_ARRAY_TEXTURES_SUPPORT)
@group(0) @binding(5) public var directional_shadow_textures: texture_depth_2d;
@else
@group(0) @binding(5) public var directional_shadow_textures: texture_depth_2d_array;
@group(0) @binding(6) public var directional_shadow_textures_comparison_sampler: sampler_comparison;
@if(PCSS_SAMPLERS_AVAILABLE)
@group(0) @binding(7) public var directional_shadow_textures_linear_sampler: sampler;

@if(AVAILABLE_STORAGE_BUFFER_BINDINGS__GE_3) {
@group(0) @binding(8) public var<storage> clustered_lights: types::ClusteredLights;
@group(0) @binding(9) public var<storage> clusterable_object_index_lists: types::ClusterableObjectIndexLists;
@group(0) @binding(10) public var<storage> cluster_offsets_and_counts: types::ClusterOffsetsAndCounts;
}
@if(!AVAILABLE_STORAGE_BUFFER_BINDINGS__GE_3) {
@group(0) @binding(8) public var<uniform> clustered_lights: types::ClusteredLights;
@group(0) @binding(9) public var<uniform> clusterable_object_index_lists: types::ClusterableObjectIndexLists;
@group(0) @binding(10) public var<uniform> cluster_offsets_and_counts: types::ClusterOffsetsAndCounts;
}

@group(0) @binding(11) public var<uniform> globals: Globals;
@group(0) @binding(12) public var<uniform> light_probes: types::LightProbes;

@if(DISTANCE_FOG)
@group(0) @binding(13) public var<uniform> fog: types::Fog;

public const VISIBILITY_RANGE_UNIFORM_BUFFER_SIZE: u32 = 64u;
@if(AVAILABLE_STORAGE_BUFFER_BINDINGS__GE_6)
@group(0) @binding(14) public var<storage> visibility_ranges: array<vec4<f32>>;
@else
@group(0) @binding(14) public var<uniform> visibility_ranges: array<vec4<f32>, VISIBILITY_RANGE_UNIFORM_BUFFER_SIZE>;

@if(SCREEN_SPACE_REFLECTIONS)
@group(0) @binding(15) public var<uniform> ssr_settings: types::ScreenSpaceReflectionsSettings;

@if(CONTACT_SHADOWS)
@group(0) @binding(16) public var<uniform> contact_shadows_settings: types::ContactShadowsSettings;

@if(SCREEN_SPACE_AMBIENT_OCCLUSION)
@group(0) @binding(17) public var screen_space_ambient_occlusion_texture: texture_2d<f32>;

@if(TONEMAP_IN_SHADER) {
// NB: If you change these, make sure to update `tonemapping.wesl` too.
@group(0) @binding(18) public var dt_lut_texture: texture_3d<f32>;
@group(0) @binding(19) public var dt_lut_sampler: sampler;
}

@if(MULTISAMPLED && DEPTH_PREPASS)
@group(0) @binding(20) public var depth_prepass_texture: texture_depth_multisampled_2d;
@if(MULTISAMPLED && NORMAL_PREPASS)
@group(0) @binding(21) public var normal_prepass_texture: texture_multisampled_2d<f32>;
@if(MULTISAMPLED && MOTION_VECTOR_PREPASS)
@group(0) @binding(22) public var motion_vector_prepass_texture: texture_multisampled_2d<f32>;

@if(!MULTISAMPLED && DEPTH_PREPASS)
@group(0) @binding(20) public var depth_prepass_texture: texture_depth_2d;
@if(!MULTISAMPLED && NORMAL_PREPASS)
@group(0) @binding(21) public var normal_prepass_texture: texture_2d<f32>;
@if(!MULTISAMPLED && MOTION_VECTOR_PREPASS)
@group(0) @binding(22) public var motion_vector_prepass_texture: texture_2d<f32>;

@if(DEFERRED_PREPASS)
@group(0) @binding(23) public var deferred_prepass_texture: texture_2d<u32>;

@if (VIEW_TRANSMISSION_TEXTURE) {
@group(0) @binding(24) public var view_transmission_texture: texture_2d<f32>;
@group(0) @binding(25) public var view_transmission_sampler: sampler;
}

@if(OIT_ENABLED) {
@group(0) @binding(26) public var<uniform> oit_settings: types::OrderIndependentTransparencySettings;
@group(0) @binding(27) public var<uniform> oit_nodes_capacity: u32;
@group(0) @binding(28) public var<storage, read_write> oit_nodes: array<types::OitFragmentNode>;
@group(0) @binding(29) public var<storage, read_write> oit_heads: array<atomic<u32>>;
@group(0) @binding(30) public var<storage, read_write> oit_atomic_counter: atomic<u32>;
}

@if(ATMOSPHERE) {
@group(0) @binding(31) public var atmosphere_transmittance_texture: texture_2d<f32>;
@group(0) @binding(32) public var atmosphere_transmittance_sampler: sampler;
@group(0) @binding(33) public var<storage> atmosphere: atmosphere_types::Atmosphere;
}

@if(BLUE_NOISE_TEXTURE)
@group(0) @binding(34) public var blue_noise_texture: texture_2d_array<f32>;

@if(AREA_LIGHT_LUTS) {
@group(0) @binding(35) public var area_light_luts: texture_2d_array<f32>;
@group(0) @binding(36) public var area_light_luts_sampler: sampler;
}

@if(DFG_LUT) {
@group(0) @binding(37) public var dfg_lut: texture_2d<f32>;
@group(0) @binding(38) public var dfg_lut_sampler: sampler;
}

@if(ENVIRONMENT_MAP && MULTIPLE_LIGHT_PROBES_IN_ARRAY) {
@group(1) @binding(0) public var diffuse_environment_maps: binding_array<texture_cube<f32>, 8u>;
@group(1) @binding(1) public var specular_environment_maps: binding_array<texture_cube<f32>, 8u>;
}
@if(ENVIRONMENT_MAP && !MULTIPLE_LIGHT_PROBES_IN_ARRAY) {
@group(1) @binding(0) public var diffuse_environment_map: texture_cube<f32>;
@group(1) @binding(1) public var specular_environment_map: texture_cube<f32>;
}
@if(ENVIRONMENT_MAP)
@group(1) @binding(2) public var environment_map_sampler: sampler;

@if(IRRADIANCE_VOLUME && IRRADIANCE_VOLUMES_ARE_USABLE && MULTIPLE_LIGHT_PROBES_IN_ARRAY)
@group(1) @binding(3) public var irradiance_volumes: binding_array<texture_3d<f32>, 8u>;
@if(IRRADIANCE_VOLUME && IRRADIANCE_VOLUMES_ARE_USABLE && !MULTIPLE_LIGHT_PROBES_IN_ARRAY)
@group(1) @binding(3) public var irradiance_volume: texture_3d<f32>;
@if(IRRADIANCE_VOLUME && IRRADIANCE_VOLUMES_ARE_USABLE)
@group(1) @binding(4) public var irradiance_volume_sampler: sampler;

@if(CLUSTERED_DECALS_ARE_USABLE) {
@group(1) @binding(5) public var<storage> clustered_decals: types::ClusteredDecals;
@group(1) @binding(6) public var clustered_decal_textures: binding_array<texture_2d<f32>, 8u>;
@group(1) @binding(7) public var clustered_decal_sampler: sampler;
}

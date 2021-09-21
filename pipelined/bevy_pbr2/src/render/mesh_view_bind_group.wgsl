[[block]]
struct View {
    view_proj: mat4x4<f32>;
    inverse_view: mat4x4<f32>;
    projection: mat4x4<f32>;
    world_position: vec3<f32>;
    near: f32;
    far: f32;
    width: f32;
    height: f32;
};

struct PointLight {
    projection: mat4x4<f32>;
    color: vec4<f32>;
    position: vec3<f32>;
    inverse_square_range: f32;
    radius: f32;
    near: f32;
    far: f32;
    // 'flags' is a bit field indicating various options. u32 is 32 bits so we have up to 32 options.
    flags: u32;
    shadow_depth_bias: f32;
    shadow_normal_bias: f32;
};

let POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT: u32 = 1u;

struct DirectionalLight {
    view_projection: mat4x4<f32>;
    color: vec4<f32>;
    direction_to_light: vec3<f32>;
    // 'flags' is a bit field indicating various options. u32 is 32 bits so we have up to 32 options.
    flags: u32;
    shadow_depth_bias: f32;
    shadow_normal_bias: f32;
};

let DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT: u32 = 1u;

[[block]]
struct Lights {
    // NOTE: this array size must be kept in sync with the constants defined bevy_pbr2/src/render/light.rs
    directional_lights: array<DirectionalLight, 1>;
    ambient_color: vec4<f32>;
    cluster_dimensions: vec4<u32>; // x/y/z dimensions
    n_directional_lights: u32;
};

[[block]]
struct PointLights {
    data: array<PointLight, 128>;
};

[[block]]
struct ClusterLightIndexLists {
    data: array<u32, 4096>; // each u32 contains 4 u8 indices into the PointLights array
};

[[block]]
struct ClusterOffsetsAndCounts {
    data: array<u32, 4096>; // each u32 contains a 24-bit index into ClusterLightIndexLists in the high 24 bits
                            // and an 8-bit count of the number of lights in the low 8 bits
};

[[group(0), binding(0)]]
var<uniform> view: View;
[[group(0), binding(1)]]
var<uniform> lights: Lights;
[[group(0), binding(2)]]
var point_shadow_textures: texture_depth_cube_array;
[[group(0), binding(3)]]
var point_shadow_textures_sampler: sampler_comparison;
[[group(0), binding(4)]]
var directional_shadow_textures: texture_depth_2d_array;
[[group(0), binding(5)]]
var directional_shadow_textures_sampler: sampler_comparison;
[[group(0), binding(6)]]
var point_lights: PointLights;
[[group(0), binding(7)]]
var cluster_light_index_lists: ClusterLightIndexLists;
[[group(0), binding(8)]]
var cluster_offsets_and_counts: ClusterOffsetsAndCounts;

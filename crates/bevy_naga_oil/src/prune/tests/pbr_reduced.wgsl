struct mesh_vertex_output__MeshVertexOutput {
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct pbr_types__StandardMaterial {
    base_color: vec4<f32>,
    emissive: vec4<f32>,
    perceptual_roughness: f32,
    metallic: f32,
    reflectance: f32,
    flags: u32,
    alpha_cutoff: f32,
}

struct mesh_types__Mesh {
    model: mat4x4<f32>,
    inverse_transpose_model: mat4x4<f32>,
    flags: u32,
}

struct mesh_view_types__View {
    view_proj: mat4x4<f32>,
    inverse_view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    inverse_view: mat4x4<f32>,
    projection: mat4x4<f32>,
    inverse_projection: mat4x4<f32>,
    world_position: vec3<f32>,
    width: f32,
    height: f32,
}

struct mesh_view_types__PointLight {
    light_custom_data: vec4<f32>,
    color_inverse_square_range: vec4<f32>,
    position_radius: vec4<f32>,
    flags: u32,
    shadow_depth_bias: f32,
    shadow_normal_bias: f32,
    spot_light_tan_angle: f32,
}

struct mesh_view_types__DirectionalLight {
    view_projection: mat4x4<f32>,
    color: vec4<f32>,
    direction_to_light: vec3<f32>,
    flags: u32,
    shadow_depth_bias: f32,
    shadow_normal_bias: f32,
}

struct mesh_view_types__Lights {
    directional_lights: array<mesh_view_types__DirectionalLight,1u>,
    ambient_color: vec4<f32>,
    cluster_dimensions: vec4<u32>,
    cluster_factors: vec4<f32>,
    n_directional_lights: u32,
    spot_light_shadowmap_offset: i32,
}

struct mesh_view_types__PointLights {
    data: array<mesh_view_types__PointLight>,
}

struct mesh_view_types__ClusterLightIndexLists {
    data: array<u32>,
}

struct mesh_view_types__ClusterOffsetsAndCounts {
    data: array<vec4<u32>>,
}

struct pbr_functions__PbrInput {
    material: pbr_types__StandardMaterial,
    occlusion: f32,
    frag_coord: vec4<f32>,
    world_position: vec4<f32>,
    world_normal: vec3<f32>,
    N: vec3<f32>,
    V: vec3<f32>,
    is_orthographic: bool,
}

const pbr_types__STANDARD_MATERIAL_FLAGS_EMISSIVE_TEXTURE_BIT: u32 = 2u;

const pbr_types__STANDARD_MATERIAL_FLAGS_UNLIT_BIT: u32 = 32u;

const pbr_types__STANDARD_MATERIAL_FLAGS_ALPHA_MODE_BLEND: u32 = 256u;

const pbr_types__STANDARD_MATERIAL_FLAGS_TWO_COMPONENT_NORMAL_MAP: u32 = 512u;

const pbr_types__STANDARD_MATERIAL_FLAGS_ALPHA_MODE_MASK: u32 = 128u;

const pbr_types__STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT: u32 = 16u;

const pbr_types__STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT: u32 = 1u;

const pbr_types__STANDARD_MATERIAL_FLAGS_METALLIC_ROUGHNESS_TEXTURE_BIT: u32 = 4u;

const pbr_types__STANDARD_MATERIAL_FLAGS_OCCLUSION_TEXTURE_BIT: u32 = 8u;

const pbr_types__STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE: u32 = 64u;

const pbr_types__STANDARD_MATERIAL_FLAGS_FLIP_NORMAL_MAP_Y: u32 = 1024u;

const mesh_types__MESH_FLAGS_SHADOW_RECEIVER_BIT: u32 = 1u;

const mesh_view_types__POINT_LIGHT_FLAGS_SPOT_LIGHT_Y_NEGATIVE: u32 = 2u;

const mesh_view_types__DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT: u32 = 1u;

const mesh_view_types__POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT: u32 = 1u;

const utils__PI: f32 = 3.1415927410125732;

const clustered_forward__CLUSTER_COUNT_SIZE: u32 = 9u;

@group(2) @binding(0) 
var<uniform> mesh_bindings__mesh: mesh_types__Mesh;
@group(0) @binding(0) 
var<uniform> mesh_view_bindings__view: mesh_view_types__View;
@group(0) @binding(2) 
var mesh_view_bindings__point_shadow_textures: texture_depth_cube_array;
@group(0) @binding(5) 
var mesh_view_bindings__directional_shadow_textures_sampler: sampler_comparison;
@group(0) @binding(6) 
var<storage> mesh_view_bindings__point_lights: mesh_view_types__PointLights;
@group(0) @binding(1) 
var<uniform> mesh_view_bindings__lights: mesh_view_types__Lights;
@group(0) @binding(3) 
var mesh_view_bindings__point_shadow_textures_sampler: sampler_comparison;
@group(0) @binding(4) 
var mesh_view_bindings__directional_shadow_textures: texture_depth_2d_array;
@group(0) @binding(7) 
var<storage> mesh_view_bindings__cluster_light_index_lists: mesh_view_types__ClusterLightIndexLists;
@group(0) @binding(8) 
var<storage> mesh_view_bindings__cluster_offsets_and_counts: mesh_view_types__ClusterOffsetsAndCounts;
@group(1) @binding(8) 
var pbr_bindings__occlusion_sampler: sampler;
@group(1) @binding(0) 
var<uniform> pbr_bindings__material: pbr_types__StandardMaterial;
@group(1) @binding(3) 
var pbr_bindings__emissive_texture: texture_2d<f32>;
@group(1) @binding(1) 
var pbr_bindings__base_color_texture: texture_2d<f32>;
@group(1) @binding(5) 
var pbr_bindings__metallic_roughness_texture: texture_2d<f32>;
@group(1) @binding(4) 
var pbr_bindings__emissive_sampler: sampler;
@group(1) @binding(6) 
var pbr_bindings__metallic_roughness_sampler: sampler;
@group(1) @binding(2) 
var pbr_bindings__base_color_sampler: sampler;
@group(1) @binding(10) 
var pbr_bindings__normal_map_sampler: sampler;
@group(1) @binding(9) 
var pbr_bindings__normal_map_texture: texture_2d<f32>;
@group(1) @binding(7) 
var pbr_bindings__occlusion_texture: texture_2d<f32>;


fn pbr_functions__pbr(in: pbr_functions__PbrInput) -> vec4<f32> {
    var output_color_2: vec4<f32>;
    output_color_2 = in.material.base_color;

    if ((in.material.flags & pbr_types__STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE) != 0u) {
        output_color_2.w = 1.0;
    } else {
        if ((in.material.flags & pbr_types__STANDARD_MATERIAL_FLAGS_ALPHA_MODE_MASK) != 0u) {
            let _e52: f32 = output_color_2.w;
            if (_e52 >= in.material.alpha_cutoff) {
                output_color_2.w = 1.0;
            } else {
                discard;
            }
        }
    }

    return output_color_2;
}

@fragment 
fn fragment(mesh: mesh_vertex_output__MeshVertexOutput, @builtin(front_facing) is_front: bool, @builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    var output_color: vec4<f32>;
    var pbr_input: pbr_functions__PbrInput;

    output_color = pbr_bindings__material.base_color;

    if ((pbr_bindings__material.flags & pbr_types__STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u) {
        pbr_input.material.base_color = output_color;
        pbr_input.material.flags = pbr_bindings__material.flags;
        pbr_input.material.metallic = 15.0; // unused
        pbr_input.V = vec3<f32>(1.0, 2.0, 3.0); // unused
        pbr_input.material.alpha_cutoff = pbr_bindings__material.alpha_cutoff;

        output_color = pbr_functions__pbr(pbr_input);
    }
    return output_color;
}

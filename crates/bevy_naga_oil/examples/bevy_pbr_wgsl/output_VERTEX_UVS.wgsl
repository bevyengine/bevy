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

fn pbr_types__standard_material_new() -> pbr_types__StandardMaterial {
    var material: pbr_types__StandardMaterial;

    material.base_color = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    material.emissive = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    material.perceptual_roughness = 0.08900000154972076;
    material.metallic = 0.009999999776482582;
    material.reflectance = 0.5;
    material.flags = pbr_types__STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE;
    material.alpha_cutoff = 0.5;
    let _e33: pbr_types__StandardMaterial = material;
    return _e33;
}

fn utils__saturate(value: f32) -> f32 {
    return clamp(value, 0.0, 1.0);
}

fn utils__hsv2rgb(hue: f32, saturation: f32, value_1: f32) -> vec3<f32> {
    let rgb: vec3<f32> = clamp((abs((((vec3<f32>((hue * 6.0)) + vec3<f32>(0.0, 4.0, 2.0)) % vec3<f32>(6.0)) - vec3<f32>(3.0))) - vec3<f32>(1.0)), vec3<f32>(0.0), vec3<f32>(1.0));
    return (value_1 * mix(vec3<f32>(1.0), rgb, vec3<f32>(saturation)));
}

fn utils__random1D(s: f32) -> f32 {
    return fract((sin((s * 12.989800453186035)) * 43758.546875));
}

fn lighting__getDistanceAttenuation(distanceSquare: f32, inverseRangeSquared: f32) -> f32 {
    let factor: f32 = (distanceSquare * inverseRangeSquared);
    let _e10: f32 = utils__saturate((1.0 - (factor * factor)));
    let attenuation: f32 = (_e10 * _e10);
    return ((attenuation * 1.0) / max(distanceSquare, 9.999999747378752e-5));
}

fn lighting__D_GGX(roughness: f32, NoH: f32, h: vec3<f32>) -> f32 {
    let oneMinusNoHSquared: f32 = (1.0 - (NoH * NoH));
    let a: f32 = (NoH * roughness);
    let k: f32 = (roughness / (oneMinusNoHSquared + (a * a)));
    let d: f32 = ((k * k) * (1.0 / utils__PI));
    return d;
}

fn lighting__V_SmithGGXCorrelated(roughness_1: f32, NoV: f32, NoL: f32) -> f32 {
    let a2_: f32 = (roughness_1 * roughness_1);
    let lambdaV: f32 = (NoL * sqrt((((NoV - (a2_ * NoV)) * NoV) + a2_)));
    let lambdaL: f32 = (NoV * sqrt((((NoL - (a2_ * NoL)) * NoL) + a2_)));
    let v_1: f32 = (0.5 / (lambdaV + lambdaL));
    return v_1;
}

fn lighting__F_Schlick_vec(f0_: vec3<f32>, f90_: f32, VoH: f32) -> vec3<f32> {
    return (f0_ + ((vec3<f32>(f90_) - f0_) * pow((1.0 - VoH), 5.0)));
}

fn lighting__F_Schlick(f0_1: f32, f90_1: f32, VoH_1: f32) -> f32 {
    return (f0_1 + ((f90_1 - f0_1) * pow((1.0 - VoH_1), 5.0)));
}

fn lighting__fresnel(f0_2: vec3<f32>, LoH: f32) -> vec3<f32> {
    let _e11: f32 = utils__saturate(dot(f0_2, vec3<f32>((50.0 * 0.33000001311302185))));
    let _e12: vec3<f32> = lighting__F_Schlick_vec(f0_2, _e11, LoH);
    return _e12;
}

fn lighting__specular(f0_3: vec3<f32>, roughness_2: f32, h_1: vec3<f32>, NoV_1: f32, NoL_1: f32, NoH_1: f32, LoH_1: f32, specularIntensity: f32) -> vec3<f32> {
    let _e12: f32 = lighting__D_GGX(roughness_2, NoH_1, h_1);
    let _e13: f32 = lighting__V_SmithGGXCorrelated(roughness_2, NoV_1, NoL_1);
    let _e14: vec3<f32> = lighting__fresnel(f0_3, LoH_1);
    return (((specularIntensity * _e12) * _e13) * _e14);
}

fn lighting__Fd_Burley(roughness_3: f32, NoV_2: f32, NoL_2: f32, LoH_2: f32) -> f32 {
    let f90_2: f32 = (0.5 + (((2.0 * roughness_3) * LoH_2) * LoH_2));
    let _e15: f32 = lighting__F_Schlick(1.0, f90_2, NoL_2);
    let _e17: f32 = lighting__F_Schlick(1.0, f90_2, NoV_2);
    return ((_e15 * _e17) * (1.0 / utils__PI));
}

fn lighting__EnvBRDFApprox(f0_4: vec3<f32>, perceptual_roughness_1: f32, NoV_3: f32) -> vec3<f32> {
    let c0_: vec4<f32> = vec4<f32>(-1.0, -0.027499999850988388, -0.5720000267028809, 0.02199999988079071);
    let c1_: vec4<f32> = vec4<f32>(1.0, 0.042500000447034836, 1.0399999618530273, -0.03999999910593033);
    let r: vec4<f32> = ((perceptual_roughness_1 * c0_) + c1_);
    let a004_: f32 = ((min((r.x * r.x), exp2((-9.279999732971191 * NoV_3))) * r.x) + r.y);
    let AB: vec2<f32> = ((vec2<f32>(-1.0399999618530273, 1.0399999618530273) * a004_) + r.zw);
    return ((f0_4 * AB.x) + vec3<f32>(AB.y));
}

fn lighting__perceptualRoughnessToRoughness(perceptualRoughness: f32) -> f32 {
    let clampedPerceptualRoughness: f32 = clamp(perceptualRoughness, 0.08900000154972076, 1.0);
    return (clampedPerceptualRoughness * clampedPerceptualRoughness);
}

fn lighting__reinhard(color: vec3<f32>) -> vec3<f32> {
    return (color / (vec3<f32>(1.0) + color));
}

fn lighting__reinhard_extended(color_1: vec3<f32>, max_white: f32) -> vec3<f32> {
    let numerator: vec3<f32> = (color_1 * (vec3<f32>(1.0) + (color_1 / vec3<f32>((max_white * max_white)))));
    return (numerator / (vec3<f32>(1.0) + color_1));
}

fn lighting__luminance(v: vec3<f32>) -> f32 {
    return dot(v, vec3<f32>(0.2125999927520752, 0.7152000069618225, 0.0722000002861023));
}

fn lighting__change_luminance(c_in: vec3<f32>, l_out: f32) -> vec3<f32> {
    let _e6: f32 = lighting__luminance(c_in);
    return (c_in * (l_out / _e6));
}

fn lighting__reinhard_luminance(color_2: vec3<f32>) -> vec3<f32> {
    let _e5: f32 = lighting__luminance(color_2);
    let l_new: f32 = (_e5 / (1.0 + _e5));
    let _e9: vec3<f32> = lighting__change_luminance(color_2, l_new);
    return _e9;
}

fn lighting__reinhard_extended_luminance(color_3: vec3<f32>, max_white_l: f32) -> vec3<f32> {
    let _e6: f32 = lighting__luminance(color_3);
    let numerator_1: f32 = (_e6 * (1.0 + (_e6 / (max_white_l * max_white_l))));
    let l_new_1: f32 = (numerator_1 / (1.0 + _e6));
    let _e15: vec3<f32> = lighting__change_luminance(color_3, l_new_1);
    return _e15;
}

fn lighting__point_light(world_position: vec3<f32>, light: mesh_view_types__PointLight, roughness_4: f32, NdotV: f32, N: vec3<f32>, V: vec3<f32>, R: vec3<f32>, F0_: vec3<f32>, diffuseColor: vec3<f32>) -> vec3<f32> {
    var L: vec3<f32>;
    var H: vec3<f32>;
    var NoL_3: f32;
    var NoH_2: f32;
    var LoH_3: f32;

    let light_to_frag: vec3<f32> = (light.position_radius.xyz - world_position.xyz);
    let distance_square: f32 = dot(light_to_frag, light_to_frag);
    let _e20: f32 = lighting__getDistanceAttenuation(distance_square, light.color_inverse_square_range.w);
    let centerToRay: vec3<f32> = ((dot(light_to_frag, R) * R) - light_to_frag);
    let _e29: f32 = utils__saturate((light.position_radius.w * inverseSqrt(dot(centerToRay, centerToRay))));
    let closestPoint: vec3<f32> = (light_to_frag + (centerToRay * _e29));
    let LspecLengthInverse: f32 = inverseSqrt(dot(closestPoint, closestPoint));
    let _e40: f32 = utils__saturate((roughness_4 + ((light.position_radius.w * 0.5) * LspecLengthInverse)));
    let normalizationFactor: f32 = (roughness_4 / _e40);
    let specularIntensity_1: f32 = (normalizationFactor * normalizationFactor);
    L = (closestPoint * LspecLengthInverse);
    let _e45: vec3<f32> = L;
    H = normalize((_e45 + V));
    let _e49: vec3<f32> = L;
    let _e51: f32 = utils__saturate(dot(N, _e49));
    NoL_3 = _e51;
    let _e53: vec3<f32> = H;
    let _e55: f32 = utils__saturate(dot(N, _e53));
    NoH_2 = _e55;
    let _e57: vec3<f32> = L;
    let _e58: vec3<f32> = H;
    let _e60: f32 = utils__saturate(dot(_e57, _e58));
    LoH_3 = _e60;
    let _e62: vec3<f32> = H;
    let _e63: f32 = NoL_3;
    let _e64: f32 = NoH_2;
    let _e65: f32 = LoH_3;
    let _e66: vec3<f32> = lighting__specular(F0_, roughness_4, _e62, NdotV, _e63, _e64, _e65, specularIntensity_1);
    L = normalize(light_to_frag);
    let _e68: vec3<f32> = L;
    H = normalize((_e68 + V));
    let _e71: vec3<f32> = L;
    let _e73: f32 = utils__saturate(dot(N, _e71));
    NoL_3 = _e73;
    let _e74: vec3<f32> = H;
    let _e76: f32 = utils__saturate(dot(N, _e74));
    NoH_2 = _e76;
    let _e77: vec3<f32> = L;
    let _e78: vec3<f32> = H;
    let _e80: f32 = utils__saturate(dot(_e77, _e78));
    LoH_3 = _e80;
    let _e81: f32 = NoL_3;
    let _e82: f32 = LoH_3;
    let _e83: f32 = lighting__Fd_Burley(roughness_4, NdotV, _e81, _e82);
    let diffuse: vec3<f32> = (diffuseColor * _e83);
    let _e89: f32 = NoL_3;
    return (((diffuse + _e66) * light.color_inverse_square_range.xyz) * (_e20 * _e89));
}

fn lighting__spot_light(world_position_1: vec3<f32>, light_1: mesh_view_types__PointLight, roughness_5: f32, NdotV_1: f32, N_1: vec3<f32>, V_1: vec3<f32>, R_1: vec3<f32>, F0_1: vec3<f32>, diffuseColor_1: vec3<f32>) -> vec3<f32> {
    var spot_dir: vec3<f32>;

    let _e13: vec3<f32> = lighting__point_light(world_position_1, light_1, roughness_5, NdotV_1, N_1, V_1, R_1, F0_1, diffuseColor_1);
    spot_dir = vec3<f32>(light_1.light_custom_data.x, 0.0, light_1.light_custom_data.y);
    let _e24: f32 = spot_dir.x;
    let _e26: f32 = spot_dir.x;
    let _e30: f32 = spot_dir.z;
    let _e32: f32 = spot_dir.z;
    spot_dir.y = sqrt(((1.0 - (_e24 * _e26)) - (_e30 * _e32)));
    if ((light_1.flags & mesh_view_types__POINT_LIGHT_FLAGS_SPOT_LIGHT_Y_NEGATIVE) != 0u) {
        let _e42: f32 = spot_dir.y;
        spot_dir.y = -(_e42);
    }
    let light_to_frag_1: vec3<f32> = (light_1.position_radius.xyz - world_position_1.xyz);
    let _e48: vec3<f32> = spot_dir;
    let cd: f32 = dot(-(_e48), normalize(light_to_frag_1));
    let _e58: f32 = utils__saturate(((cd * light_1.light_custom_data.z) + light_1.light_custom_data.w));
    let spot_attenuation: f32 = (_e58 * _e58);
    return (_e13 * spot_attenuation);
}

fn lighting__directional_light(light_2: mesh_view_types__DirectionalLight, roughness_6: f32, NdotV_2: f32, normal: vec3<f32>, view: vec3<f32>, R_2: vec3<f32>, F0_2: vec3<f32>, diffuseColor_2: vec3<f32>) -> vec3<f32> {
    let incident_light: vec3<f32> = light_2.direction_to_light.xyz;
    let half_vector: vec3<f32> = normalize((incident_light + view));
    let _e17: f32 = utils__saturate(dot(normal, incident_light));
    let _e19: f32 = utils__saturate(dot(normal, half_vector));
    let _e21: f32 = utils__saturate(dot(incident_light, half_vector));
    let _e22: f32 = lighting__Fd_Burley(roughness_6, NdotV_2, _e17, _e21);
    let diffuse_1: vec3<f32> = (diffuseColor_2 * _e22);
    let _e25: vec3<f32> = lighting__specular(F0_2, roughness_6, half_vector, NdotV_2, _e17, _e19, _e21, 1.0);
    return (((_e25 + diffuse_1) * light_2.color.xyz) * _e17);
}

fn clustered_forward__view_z_to_z_slice(view_z: f32, is_orthographic: bool) -> u32 {
    var z_slice: u32 = 0u;

    if is_orthographic {
        let _e18: f32 = mesh_view_bindings__lights.cluster_factors.z;
        let _e22: f32 = mesh_view_bindings__lights.cluster_factors.w;
        z_slice = u32(floor(((view_z - _e18) * _e22)));
    } else {
        let _e30: f32 = mesh_view_bindings__lights.cluster_factors.z;
        let _e34: f32 = mesh_view_bindings__lights.cluster_factors.w;
        z_slice = u32((((log(-(view_z)) * _e30) - _e34) + 1.0));
    }
    let _e39: u32 = z_slice;
    let _e42: u32 = mesh_view_bindings__lights.cluster_dimensions.z;
    return min(_e39, (_e42 - 1u));
}

fn clustered_forward__fragment_cluster_index(frag_coord_1: vec2<f32>, view_z_1: f32, is_orthographic_1: bool) -> u32 {
    let _e16: vec4<f32> = mesh_view_bindings__lights.cluster_factors;
    let xy: vec2<u32> = vec2<u32>(floor((frag_coord_1 * _e16.xy)));
    let _e21: u32 = clustered_forward__view_z_to_z_slice(view_z_1, is_orthographic_1);
    let _e25: u32 = mesh_view_bindings__lights.cluster_dimensions.x;
    let _e31: u32 = mesh_view_bindings__lights.cluster_dimensions.z;
    let _e36: u32 = mesh_view_bindings__lights.cluster_dimensions.w;
    return min(((((xy.y * _e25) + xy.x) * _e31) + _e21), (_e36 - 1u));
}

fn clustered_forward__unpack_offset_and_counts(cluster_index: u32) -> vec3<u32> {
    let _e16: vec4<u32> = mesh_view_bindings__cluster_offsets_and_counts.data[cluster_index];
    return _e16.xyz;
}

fn clustered_forward__get_light_id(index: u32) -> u32 {
    let _e16: u32 = mesh_view_bindings__cluster_light_index_lists.data[index];
    return _e16;
}

fn clustered_forward__cluster_debug_visualization(output_color_1: vec4<f32>, view_z_2: f32, is_orthographic_2: bool, offset_and_counts: vec3<u32>, cluster_index_1: u32) -> vec4<f32> {
    return output_color_1;
}

fn shadows__fetch_point_shadow(light_id: u32, frag_position: vec4<f32>, surface_normal: vec3<f32>) -> f32 {
    let light_3: mesh_view_types__PointLight = mesh_view_bindings__point_lights.data[light_id];
    let surface_to_light: vec3<f32> = (light_3.position_radius.xyz - frag_position.xyz);
    let surface_to_light_abs: vec3<f32> = abs(surface_to_light);
    let distance_to_light: f32 = max(surface_to_light_abs.x, max(surface_to_light_abs.y, surface_to_light_abs.z));
    let normal_offset: vec3<f32> = ((light_3.shadow_normal_bias * distance_to_light) * surface_normal.xyz);
    let depth_offset: vec3<f32> = (light_3.shadow_depth_bias * normalize(surface_to_light.xyz));
    let offset_position: vec3<f32> = ((frag_position.xyz + normal_offset) + depth_offset);
    let frag_ls: vec3<f32> = (light_3.position_radius.xyz - offset_position.xyz);
    let abs_position_ls: vec3<f32> = abs(frag_ls);
    let major_axis_magnitude: f32 = max(abs_position_ls.x, max(abs_position_ls.y, abs_position_ls.z));
    let zw: vec2<f32> = ((-(major_axis_magnitude) * light_3.light_custom_data.xy) + light_3.light_custom_data.zw);
    let depth: f32 = (zw.x / zw.y);
    let _e60: f32 = textureSampleCompareLevel(mesh_view_bindings__point_shadow_textures, mesh_view_bindings__point_shadow_textures_sampler, frag_ls, i32(light_id), depth);
    return _e60;
}

fn shadows__fetch_spot_shadow(light_id_1: u32, frag_position_1: vec4<f32>, surface_normal_1: vec3<f32>) -> f32 {
    var spot_dir_1: vec3<f32>;
    var sign: f32 = -1.0;

    let light_4: mesh_view_types__PointLight = mesh_view_bindings__point_lights.data[light_id_1];
    let surface_to_light_1: vec3<f32> = (light_4.position_radius.xyz - frag_position_1.xyz);
    spot_dir_1 = vec3<f32>(light_4.light_custom_data.x, 0.0, light_4.light_custom_data.y);
    let _e32: f32 = spot_dir_1.x;
    let _e34: f32 = spot_dir_1.x;
    let _e38: f32 = spot_dir_1.z;
    let _e40: f32 = spot_dir_1.z;
    spot_dir_1.y = sqrt(((1.0 - (_e32 * _e34)) - (_e38 * _e40)));
    if ((light_4.flags & mesh_view_types__POINT_LIGHT_FLAGS_SPOT_LIGHT_Y_NEGATIVE) != 0u) {
        let _e50: f32 = spot_dir_1.y;
        spot_dir_1.y = -(_e50);
    }
    let _e52: vec3<f32> = spot_dir_1;
    let fwd: vec3<f32> = -(_e52);
    let distance_to_light_1: f32 = dot(fwd, surface_to_light_1);
    let offset_position_1: vec3<f32> = ((-(surface_to_light_1) + (light_4.shadow_depth_bias * normalize(surface_to_light_1))) + ((surface_normal_1.xyz * light_4.shadow_normal_bias) * distance_to_light_1));
    if (fwd.z >= 0.0) {
        sign = 1.0;
    }
    let _e73: f32 = sign;
    let a_1: f32 = (-1.0 / (fwd.z + _e73));
    let b: f32 = ((fwd.x * fwd.y) * a_1);
    let _e81: f32 = sign;
    let _e88: f32 = sign;
    let _e90: f32 = sign;
    let up_dir: vec3<f32> = vec3<f32>((1.0 + (((_e81 * fwd.x) * fwd.x) * a_1)), (_e88 * b), (-(_e90) * fwd.x));
    let _e96: f32 = sign;
    let right_dir: vec3<f32> = vec3<f32>(-(b), (-(_e96) - ((fwd.y * fwd.y) * a_1)), fwd.y);
    let light_inv_rot: mat3x3<f32> = mat3x3<f32>(right_dir, up_dir, fwd);
    let projected_position: vec3<f32> = (offset_position_1 * light_inv_rot);
    let f_div_minus_z: f32 = (1.0 / (light_4.spot_light_tan_angle * -(projected_position.z)));
    let shadow_xy_ndc: vec2<f32> = (projected_position.xy * f_div_minus_z);
    let shadow_uv: vec2<f32> = ((shadow_xy_ndc * vec2<f32>(0.5, -0.5)) + vec2<f32>(0.5, 0.5));
    let depth_1: f32 = (0.10000000149011612 / -(projected_position.z));
    let _e129: i32 = mesh_view_bindings__lights.spot_light_shadowmap_offset;
    let _e131: f32 = textureSampleCompareLevel(mesh_view_bindings__directional_shadow_textures, mesh_view_bindings__directional_shadow_textures_sampler, shadow_uv, (i32(light_id_1) + _e129), depth_1);
    return _e131;
}

fn shadows__fetch_directional_shadow(light_id_2: u32, frag_position_2: vec4<f32>, surface_normal_2: vec3<f32>) -> f32 {
    let light_5: mesh_view_types__DirectionalLight = mesh_view_bindings__lights.directional_lights[light_id_2];
    let normal_offset_1: vec3<f32> = (light_5.shadow_normal_bias * surface_normal_2.xyz);
    let depth_offset_1: vec3<f32> = (light_5.shadow_depth_bias * light_5.direction_to_light.xyz);
    let offset_position_2: vec4<f32> = vec4<f32>(((frag_position_2.xyz + normal_offset_1) + depth_offset_1), frag_position_2.w);
    let offset_position_clip: vec4<f32> = (light_5.view_projection * offset_position_2);
    if (offset_position_clip.w <= 0.0) {
        return 1.0;
    }
    let offset_position_ndc: vec3<f32> = (offset_position_clip.xyz / vec3<f32>(offset_position_clip.w));
    if ((any((offset_position_ndc.xy < vec2<f32>(-1.0))) || (offset_position_ndc.z < 0.0)) || any((offset_position_ndc > vec3<f32>(1.0)))) {
        return 1.0;
    }
    let flip_correction: vec2<f32> = vec2<f32>(0.5, -0.5);
    let light_local: vec2<f32> = ((offset_position_ndc.xy * flip_correction) + vec2<f32>(0.5, 0.5));
    let depth_2: f32 = offset_position_ndc.z;
    let _e66: f32 = textureSampleCompareLevel(mesh_view_bindings__directional_shadow_textures, mesh_view_bindings__directional_shadow_textures_sampler, light_local, i32(light_id_2), depth_2);
    return _e66;
}

fn pbr_functions__prepare_normal(standard_material_flags: u32, world_normal: vec3<f32>, uv: vec2<f32>, is_front_1: bool) -> vec3<f32> {
    var N_2: vec3<f32>;

    N_2 = normalize(world_normal);
    if ((standard_material_flags & pbr_types__STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u) {
        if !(is_front_1) {
            let _e37: vec3<f32> = N_2;
            N_2 = -(_e37);
        }
    }
    let _e39: vec3<f32> = N_2;
    return _e39;
}

fn pbr_functions__calculate_view(world_position_2: vec4<f32>, is_orthographic_3: bool) -> vec3<f32> {
    var V_2: vec3<f32>;

    if is_orthographic_3 {
        let _e34: f32 = mesh_view_bindings__view.view_proj[0][2];
        let _e39: f32 = mesh_view_bindings__view.view_proj[1][2];
        let _e44: f32 = mesh_view_bindings__view.view_proj[2][2];
        V_2 = normalize(vec3<f32>(_e34, _e39, _e44));
    } else {
        let _e48: vec3<f32> = mesh_view_bindings__view.world_position;
        V_2 = normalize((_e48.xyz - world_position_2.xyz));
    }
    let _e53: vec3<f32> = V_2;
    return _e53;
}

fn pbr_functions__pbr_input_new() -> pbr_functions__PbrInput {
    var pbr_input_1: pbr_functions__PbrInput;

    let _e29: pbr_types__StandardMaterial = pbr_types__standard_material_new();
    pbr_input_1.material = _e29;
    pbr_input_1.occlusion = 1.0;
    pbr_input_1.frag_coord = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    pbr_input_1.world_position = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    pbr_input_1.world_normal = vec3<f32>(0.0, 0.0, 1.0);
    pbr_input_1.is_orthographic = false;
    pbr_input_1.N = vec3<f32>(0.0, 0.0, 1.0);
    pbr_input_1.V = vec3<f32>(1.0, 0.0, 0.0);
    let _e61: pbr_functions__PbrInput = pbr_input_1;
    return _e61;
}

fn pbr_functions__pbr(in: pbr_functions__PbrInput) -> vec4<f32> {
    var output_color_2: vec4<f32>;
    var light_accum: vec3<f32>;
    var i: u32;
    var shadow: f32;
    var i_1: u32;
    var shadow_1: f32;
    var i_2: u32 = 0u;
    var shadow_2: f32;

    output_color_2 = in.material.base_color;
    let emissive_1: vec4<f32> = in.material.emissive;
    let metallic_1: f32 = in.material.metallic;
    let perceptual_roughness_2: f32 = in.material.perceptual_roughness;
    let _e37: f32 = lighting__perceptualRoughnessToRoughness(perceptual_roughness_2);
    let occlusion_1: f32 = in.occlusion;
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
    let NdotV_3: f32 = max(dot(in.N, in.V), 9.999999747378752e-5);
    let reflectance: f32 = in.material.reflectance;
    let _e71: vec4<f32> = output_color_2;
    let F0_3: vec3<f32> = (vec3<f32>((((0.1599999964237213 * reflectance) * reflectance) * (1.0 - metallic_1))) + (_e71.xyz * metallic_1));
    let _e76: vec4<f32> = output_color_2;
    let diffuse_color: vec3<f32> = (_e76.xyz * (1.0 - metallic_1));
    let R_3: vec3<f32> = reflect(-(in.V), in.N);
    light_accum = vec3<f32>(0.0);
    let _e92: f32 = mesh_view_bindings__view.inverse_view[0][2];
    let _e97: f32 = mesh_view_bindings__view.inverse_view[1][2];
    let _e102: f32 = mesh_view_bindings__view.inverse_view[2][2];
    let _e107: f32 = mesh_view_bindings__view.inverse_view[3][2];
    let view_z_3: f32 = dot(vec4<f32>(_e92, _e97, _e102, _e107), in.world_position);
    let _e114: u32 = clustered_forward__fragment_cluster_index(in.frag_coord.xy, view_z_3, in.is_orthographic);
    let _e115: vec3<u32> = clustered_forward__unpack_offset_and_counts(_e114);
    i = _e115.x;
    loop {
        let _e119: u32 = i;
        if (_e119 < (_e115.x + _e115.y)) {
        } else {
            break;
        }
        let _e129: u32 = i;
        let _e130: u32 = clustered_forward__get_light_id(_e129);
        let light_6: mesh_view_types__PointLight = mesh_view_bindings__point_lights.data[_e130];
        shadow = 1.0;
        let _e137: u32 = mesh_bindings__mesh.flags;
        if (((_e137 & mesh_types__MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u) && ((light_6.flags & mesh_view_types__POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u)) {
            let _e148: f32 = shadows__fetch_point_shadow(_e130, in.world_position, in.world_normal);
            shadow = _e148;
        }
        let _e153: vec3<f32> = lighting__point_light(in.world_position.xyz, light_6, _e37, NdotV_3, in.N, in.V, R_3, F0_3, diffuse_color);
        let _e154: vec3<f32> = light_accum;
        let _e155: f32 = shadow;
        light_accum = (_e154 + (_e153 * _e155));
        continuing {
            let _e126: u32 = i;
            i = (_e126 + 1u);
        }
    }
    i_1 = (_e115.x + _e115.y);
    loop {
        let _e164: u32 = i_1;
        if (_e164 < ((_e115.x + _e115.y) + _e115.z)) {
        } else {
            break;
        }
        let _e177: u32 = i_1;
        let _e178: u32 = clustered_forward__get_light_id(_e177);
        let light_7: mesh_view_types__PointLight = mesh_view_bindings__point_lights.data[_e178];
        shadow_1 = 1.0;
        let _e185: u32 = mesh_bindings__mesh.flags;
        if (((_e185 & mesh_types__MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u) && ((light_7.flags & mesh_view_types__POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u)) {
            let _e196: f32 = shadows__fetch_spot_shadow(_e178, in.world_position, in.world_normal);
            shadow_1 = _e196;
        }
        let _e201: vec3<f32> = lighting__spot_light(in.world_position.xyz, light_7, _e37, NdotV_3, in.N, in.V, R_3, F0_3, diffuse_color);
        let _e202: vec3<f32> = light_accum;
        let _e203: f32 = shadow_1;
        light_accum = (_e202 + (_e201 * _e203));
        continuing {
            let _e174: u32 = i_1;
            i_1 = (_e174 + 1u);
        }
    }
    let n_directional_lights: u32 = mesh_view_bindings__lights.n_directional_lights;
    loop {
        let _e210: u32 = i_2;
        if (_e210 < n_directional_lights) {
        } else {
            break;
        }
        let _e216: u32 = i_2;
        let light_8: mesh_view_types__DirectionalLight = mesh_view_bindings__lights.directional_lights[_e216];
        shadow_2 = 1.0;
        let _e222: u32 = mesh_bindings__mesh.flags;
        if (((_e222 & mesh_types__MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u) && ((light_8.flags & mesh_view_types__DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u)) {
            let _e231: u32 = i_2;
            let _e234: f32 = shadows__fetch_directional_shadow(_e231, in.world_position, in.world_normal);
            shadow_2 = _e234;
        }
        let _e237: vec3<f32> = lighting__directional_light(light_8, _e37, NdotV_3, in.N, in.V, R_3, F0_3, diffuse_color);
        let _e238: vec3<f32> = light_accum;
        let _e239: f32 = shadow_2;
        light_accum = (_e238 + (_e237 * _e239));
        continuing {
            let _e212: u32 = i_2;
            i_2 = (_e212 + 1u);
        }
    }
    let _e243: vec3<f32> = lighting__EnvBRDFApprox(diffuse_color, 1.0, NdotV_3);
    let _e244: vec3<f32> = lighting__EnvBRDFApprox(F0_3, perceptual_roughness_2, NdotV_3);
    let _e245: vec3<f32> = light_accum;
    let _e248: vec4<f32> = mesh_view_bindings__lights.ambient_color;
    let _e255: f32 = output_color_2.w;
    let _e259: f32 = output_color_2.w;
    output_color_2 = vec4<f32>(((_e245 + (((_e243 + _e244) * _e248.xyz) * occlusion_1)) + (emissive_1.xyz * _e255)), _e259);
    let _e261: vec4<f32> = output_color_2;
    let _e263: vec4<f32> = clustered_forward__cluster_debug_visualization(_e261, view_z_3, in.is_orthographic, _e115, _e114);
    output_color_2 = _e263;
    let _e264: vec4<f32> = output_color_2;
    return _e264;
}

fn pbr_functions__tone_mapping(in_1: vec4<f32>) -> vec4<f32> {
    let _e29: vec3<f32> = lighting__reinhard_luminance(in_1.xyz);
    return vec4<f32>(_e29, in_1.w);
}

@fragment 
fn fragment(mesh: mesh_vertex_output__MeshVertexOutput, @builtin(front_facing) is_front: bool, @builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    var output_color: vec4<f32>;
    var pbr_input: pbr_functions__PbrInput;
    var emissive: vec4<f32>;
    var metallic: f32;
    var perceptual_roughness: f32;
    var occlusion: f32;

    let _e42: vec4<f32> = pbr_bindings__material.base_color;
    output_color = _e42;
    let _e45: u32 = pbr_bindings__material.flags;
    if ((_e45 & pbr_types__STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT) != 0u) {
        let _e49: vec4<f32> = output_color;
        let _e51: vec4<f32> = textureSample(pbr_bindings__base_color_texture, pbr_bindings__base_color_sampler, mesh.uv);
        output_color = (_e49 * _e51);
    }
    let _e54: u32 = pbr_bindings__material.flags;
    if ((_e54 & pbr_types__STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u) {
        let _e61: vec4<f32> = output_color;
        pbr_input.material.base_color = _e61;
        let _e65: f32 = pbr_bindings__material.reflectance;
        pbr_input.material.reflectance = _e65;
        let _e69: u32 = pbr_bindings__material.flags;
        pbr_input.material.flags = _e69;
        let _e73: f32 = pbr_bindings__material.alpha_cutoff;
        pbr_input.material.alpha_cutoff = _e73;
        let _e75: vec4<f32> = pbr_bindings__material.emissive;
        emissive = _e75;
        let _e78: u32 = pbr_bindings__material.flags;
        if ((_e78 & pbr_types__STANDARD_MATERIAL_FLAGS_EMISSIVE_TEXTURE_BIT) != 0u) {
            let _e82: vec4<f32> = emissive;
            let _e85: vec4<f32> = textureSample(pbr_bindings__emissive_texture, pbr_bindings__emissive_sampler, mesh.uv);
            emissive = vec4<f32>((_e82.xyz * _e85.xyz), 1.0);
        }
        let _e92: vec4<f32> = emissive;
        pbr_input.material.emissive = _e92;
        let _e94: f32 = pbr_bindings__material.metallic;
        metallic = _e94;
        let _e97: f32 = pbr_bindings__material.perceptual_roughness;
        perceptual_roughness = _e97;
        let _e100: u32 = pbr_bindings__material.flags;
        if ((_e100 & pbr_types__STANDARD_MATERIAL_FLAGS_METALLIC_ROUGHNESS_TEXTURE_BIT) != 0u) {
            let metallic_roughness: vec4<f32> = textureSample(pbr_bindings__metallic_roughness_texture, pbr_bindings__metallic_roughness_sampler, mesh.uv);
            let _e106: f32 = metallic;
            metallic = (_e106 * metallic_roughness.z);
            let _e109: f32 = perceptual_roughness;
            perceptual_roughness = (_e109 * metallic_roughness.y);
        }
        let _e114: f32 = metallic;
        pbr_input.material.metallic = _e114;
        let _e117: f32 = perceptual_roughness;
        pbr_input.material.perceptual_roughness = _e117;
        occlusion = 1.0;
        let _e121: u32 = pbr_bindings__material.flags;
        if ((_e121 & pbr_types__STANDARD_MATERIAL_FLAGS_OCCLUSION_TEXTURE_BIT) != 0u) {
            let _e126: vec4<f32> = textureSample(pbr_bindings__occlusion_texture, pbr_bindings__occlusion_sampler, mesh.uv);
            occlusion = _e126.x;
        }
        let _e129: f32 = occlusion;
        pbr_input.occlusion = _e129;
        pbr_input.frag_coord = frag_coord;
        pbr_input.world_position = mesh.world_position;
        pbr_input.world_normal = mesh.world_normal;
        let _e140: f32 = mesh_view_bindings__view.projection[3][3];
        pbr_input.is_orthographic = (_e140 == 1.0);
        let _e145: u32 = pbr_bindings__material.flags;
        let _e148: vec3<f32> = pbr_functions__prepare_normal(_e145, mesh.world_normal, mesh.uv, is_front);
        pbr_input.N = _e148;
        let _e152: bool = pbr_input.is_orthographic;
        let _e153: vec3<f32> = pbr_functions__calculate_view(mesh.world_position, _e152);
        pbr_input.V = _e153;
        let _e154: pbr_functions__PbrInput = pbr_input;
        let _e155: vec4<f32> = pbr_functions__pbr(_e154);
        let _e156: vec4<f32> = pbr_functions__tone_mapping(_e155);
        output_color = _e156;
    }
    let _e157: vec4<f32> = output_color;
    return _e157;
}

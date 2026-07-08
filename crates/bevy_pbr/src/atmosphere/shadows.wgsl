//! Shadow sampling for the atmosphere pipeline.
//!
//! This module provides directional light shadow sampling for atmosphere rendering,
//! using the atmosphere bind group layout. It avoids importing mesh_view_bindings
//! which would pull in the full mesh pipeline dependencies.

#define_import_path bevy_pbr::atmosphere::shadows

#import bevy_pbr::atmosphere::bindings::{atmosphere, lights, view}

// Shadow texture bindings
#ifdef NO_ARRAY_TEXTURES_SUPPORT
@group(0) @binding(16) var directional_shadow_textures: texture_depth_2d;
#else
@group(0) @binding(16) var directional_shadow_textures: texture_depth_2d_array;
#endif
@group(0) @binding(17) var directional_shadow_textures_comparison_sampler: sampler_comparison;

fn sample_shadow_map_hardware(light_local: vec2<f32>, depth: f32, array_index: i32) -> f32 {
#ifdef NO_ARRAY_TEXTURES_SUPPORT
    return textureSampleCompare(
        directional_shadow_textures,
        directional_shadow_textures_comparison_sampler,
        light_local,
        depth,
    );
#else
    return textureSampleCompareLevel(
        directional_shadow_textures,
        directional_shadow_textures_comparison_sampler,
        light_local,
        array_index,
        depth,
    );
#endif
}

fn get_cascade_index(light_id: u32, view_z: f32) -> u32 {
    let light = &lights.directional_lights[light_id];

    for (var i: u32 = 0u; i < (*light).num_cascades; i = i + 1u) {
        if (-view_z < (*light).cascades[i].far_bound) {
            return i;
        }
    }
    return (*light).num_cascades;
}

fn world_to_directional_light_local(
    light_id: u32,
    cascade_index: u32,
    offset_position: vec4<f32>,
) -> vec4<f32> {
    let light = &lights.directional_lights[light_id];
    let cascade = &(*light).cascades[cascade_index];

    let offset_position_clip = (*cascade).clip_from_world * offset_position;
    if (offset_position_clip.w <= 0.0) {
        return vec4(0.0);
    }
    let offset_position_ndc = offset_position_clip.xyz / offset_position_clip.w;
    if (any(offset_position_ndc.xy < vec2<f32>(-1.0)) || offset_position_ndc.z < 0.0
            || any(offset_position_ndc > vec3<f32>(1.0))) {
        return vec4(0.0);
    }

    let flip_correction = vec2<f32>(0.5, -0.5);
    let light_local = offset_position_ndc.xy * flip_correction + vec2<f32>(0.5, 0.5);
    let depth = offset_position_ndc.z;

    return vec4(light_local, depth, 1.0);
}

fn sample_directional_cascade(
    light_id: u32,
    cascade_index: u32,
    frag_position: vec4<f32>,
    surface_normal: vec3<f32>,
) -> f32 {
    let light = &lights.directional_lights[light_id];
    let cascade = &(*light).cascades[cascade_index];

    let normal_offset = (*light).shadow_normal_bias * (*cascade).texel_size * surface_normal.xyz;
    let depth_offset = (*light).shadow_depth_bias * (*light).direction_to_light.xyz;
    let offset_position = vec4<f32>(frag_position.xyz + normal_offset + depth_offset, frag_position.w);

    let light_local = world_to_directional_light_local(light_id, cascade_index, offset_position);
    if (light_local.w == 0.0) {
        return 1.0;
    }

    let array_index = i32((*light).depth_texture_base_index + cascade_index);
    return sample_shadow_map_hardware(light_local.xy, light_local.z, array_index);
}

/// Converts atmosphere-space position to world space for shadow lookup.
fn atmosphere_to_world_position(atmosphere_pos: vec3<f32>) -> vec3<f32> {
    return (atmosphere.atmosphere_to_world * vec4(atmosphere_pos, 1.0)).xyz;
}

/// Samples the directional light shadow at the given atmosphere-space position.
/// Returns 1.0 for unshadowed, 0.0 for fully shadowed.
fn fetch_directional_shadow(
    light_id: u32,
    atmosphere_position: vec4<f32>,
    surface_normal: vec3<f32>,
) -> f32 {
    let world_position = vec4(atmosphere_to_world_position(atmosphere_position.xyz), 1.0);
    let view_z = (view.view_from_world * world_position).z;
    let light = &lights.directional_lights[light_id];
    let cascade_index = get_cascade_index(light_id, view_z);

    if (cascade_index >= (*light).num_cascades) {
        return 1.0;
    }

    var shadow = sample_directional_cascade(
        light_id,
        cascade_index,
        world_position,
        surface_normal,
    );

    let next_cascade_index = cascade_index + 1u;
    if (next_cascade_index < (*light).num_cascades) {
        let this_far_bound = (*light).cascades[cascade_index].far_bound;
        let next_near_bound = (1.0 - (*light).cascades_overlap_proportion) * this_far_bound;
        if (-view_z >= next_near_bound) {
            let next_shadow = sample_directional_cascade(
                light_id,
                next_cascade_index,
                world_position,
                surface_normal,
            );
            shadow = mix(shadow, next_shadow, (-view_z - next_near_bound) / (this_far_bound - next_near_bound));
        }
    }
    return shadow;
}

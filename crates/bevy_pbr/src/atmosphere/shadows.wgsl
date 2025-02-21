#define_import_path bevy_pbr::atmosphere::shadows

#import bevy_pbr::{
    atmosphere::{
        bindings::{atmosphere, settings, view, lights, directional_shadow_texture, directional_shadow_sampler},
    }
}

fn sample_shadow_map_hardware(light_local: vec2<f32>, depth: f32, array_index: i32) -> f32 {
    return textureSampleCompareLevel(
        directional_shadow_texture,
        directional_shadow_sampler,
        light_local,
        array_index,
        depth,
    );
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
    offset_position: vec4<f32>
) -> vec4<f32> {
    let light = &lights.directional_lights[light_id];
    let cascade = &(*light).cascades[cascade_index];

    let offset_position_clip = (*cascade).clip_from_world * offset_position;
    if (offset_position_clip.w <= 0.0) {
        return vec4(0.0);
    }
    let offset_position_ndc = offset_position_clip.xyz / offset_position_clip.w;
    // No shadow outside the orthographic projection volume
    if (any(offset_position_ndc.xy < vec2<f32>(-1.0)) || offset_position_ndc.z < 0.0
            || any(offset_position_ndc > vec3<f32>(1.0))) {
        return vec4(0.0);
    }

    // compute texture coordinates for shadow lookup, compensating for the Y-flip difference
    // between the NDC and texture coordinates
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

    // The normal bias is scaled to the texel size.
    let normal_offset = (*light).shadow_normal_bias * (*cascade).texel_size * surface_normal.xyz;
    let depth_offset = (*light).shadow_depth_bias * (*light).direction_to_light.xyz;
    let offset_position = vec4<f32>(frag_position.xyz + normal_offset + depth_offset, frag_position.w);

    let light_local = world_to_directional_light_local(light_id, cascade_index, offset_position);
    if (light_local.w == 0.0) {
        return 1.0;
    }

    let array_index = i32((*light).depth_texture_base_index + cascade_index);
    let texel_size = (*cascade).texel_size;

    // If soft shadows are enabled, use the PCSS path.
    // if ((*light).soft_shadow_size > 0.0) {
    //     return sample_shadow_map_pcss(
    //         light_local.xy, light_local.z, array_index, texel_size, (*light).soft_shadow_size);
    // }

    return sample_shadow_map_hardware(light_local.xy, light_local.z, array_index);
}

fn fetch_directional_shadow(light_id: u32, world_pos: vec4<f32>, surface_normal: vec3<f32>, view_z: f32) -> f32 {
    let light = &lights.directional_lights[light_id];
    let cascade_index = get_cascade_index(light_id, view_z);

    if (cascade_index >= (*light).num_cascades) {
        return 1.0;
    }

    var shadow = sample_directional_cascade(light_id, cascade_index, world_pos, surface_normal);

    // Blend with the next cascade, if there is one.
    let next_cascade_index = cascade_index + 1u;
    if (next_cascade_index < (*light).num_cascades) {
        let this_far_bound = (*light).cascades[cascade_index].far_bound;
        let next_near_bound = (1.0 - (*light).cascades_overlap_proportion) * this_far_bound;
        if (-view_z >= next_near_bound) {
            let next_shadow = sample_directional_cascade(light_id, next_cascade_index, world_pos, surface_normal);
            shadow = mix(shadow, next_shadow, (-view_z - next_near_bound) / (this_far_bound - next_near_bound));
        }
    }
    return shadow;
}

fn fetch_directional_shadow2(light_index: u32, world_pos: vec4<f32>, normal: vec3<f32>, view_z: f32) -> f32 {
    let light = &lights.directional_lights[light_index]; // Using first directional light
    
    // Get cascade index based on view_z
    var cascade_index = 0u;
    for (var i = 0u; i < (*light).num_cascades; i++) {
        if (-view_z < (*light).cascades[i].far_bound) {
            cascade_index = i;
            break;
        }
    }
    
    // Get the cascade
    let cascade = &(*light).cascades[cascade_index];
    
    // Calculate position with bias
    let normal_offset = (*light).shadow_normal_bias * (*cascade).texel_size * normal;
    let depth_offset = (*light).shadow_depth_bias * (*light).direction_to_light;
    let offset_position = vec4<f32>(world_pos.xyz + normal_offset + depth_offset, world_pos.w);
    
    // Transform to light space
    let light_local = (*cascade).clip_from_world * offset_position;
    
    // Convert to UV coordinates
    let ndc = light_local.xyz / light_local.w;
    let uv = ndc.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    
    // Early exit if outside shadow map
    if (any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0))) {
        return 1.0;
    }
    
    let depth = ndc.z;
    let array_index = i32((*light).depth_texture_base_index + cascade_index);
    
    // Sample shadow map
    return textureSampleCompareLevel(
        directional_shadow_texture,
        directional_shadow_sampler,
        uv,
        array_index,
        depth
    );
}

fn get_shadow(light_index: u32, P: vec3<f32>, ray_dir: vec3<f32>) -> f32 {
    // For raymarched volumes, we can use the ray direction as the normal
    // since we don't have surface normals
    let world_normal = -ray_dir; // Point against ray direction
    let world_pos = vec4<f32>((P + vec3<f32>(0.0, -atmosphere.bottom_radius, 0.0)) / settings.scene_units_to_m, 1.0);

    // Get view space Z coordinate for cascade selection
    let view_pos = view.view_from_world * world_pos;
    let view_z = view_pos.z;

    // Assuming we're using the first directional light (index 0)
    return fetch_directional_shadow(0u, world_pos, world_normal, view_z);
}
#define_import_path bevy_pbr::shadows

fn fetch_point_shadow(light_id: u32, frag_position: vec4<f32>, surface_normal: vec3<f32>) -> f32 {
    let light = point_lights.data[light_id];

    // because the shadow maps align with the axes and the frustum planes are at 45 degrees
    // we can get the worldspace depth by taking the largest absolute axis
    let surface_to_light = light.position_radius.xyz - frag_position.xyz;
    let surface_to_light_abs = abs(surface_to_light);
    let distance_to_light = max(surface_to_light_abs.x, max(surface_to_light_abs.y, surface_to_light_abs.z));

    // The normal bias here is already scaled by the texel size at 1 world unit from the light.
    // The texel size increases proportionally with distance from the light so multiplying by
    // distance to light scales the normal bias to the texel size at the fragment distance.
    let normal_offset = light.shadow_normal_bias * distance_to_light * surface_normal.xyz;
    let depth_offset = light.shadow_depth_bias * normalize(surface_to_light.xyz);
    let offset_position = frag_position.xyz + normal_offset + depth_offset;

    // similar largest-absolute-axis trick as above, but now with the offset fragment position
    let frag_ls = light.position_radius.xyz - offset_position.xyz;
    let abs_position_ls = abs(frag_ls);
    let major_axis_magnitude = max(abs_position_ls.x, max(abs_position_ls.y, abs_position_ls.z));

    // NOTE: These simplifications come from multiplying:
    // projection * vec4(0, 0, -major_axis_magnitude, 1.0)
    // and keeping only the terms that have any impact on the depth.
    // Projection-agnostic approach:
    let zw = -major_axis_magnitude * light.projection_lr.xy + light.projection_lr.zw;
    let depth = zw.x / zw.y;

    // do the lookup, using HW PCF and comparison
    // NOTE: Due to the non-uniform control flow above, we must use the Level variant of
    // textureSampleCompare to avoid undefined behaviour due to some of the fragments in
    // a quad (2x2 fragments) being processed not being sampled, and this messing with
    // mip-mapping functionality. The shadow maps have no mipmaps so Level just samples
    // from LOD 0.
#ifdef NO_ARRAY_TEXTURES_SUPPORT
    return textureSampleCompare(point_shadow_textures, point_shadow_textures_sampler, frag_ls, depth);
#else
    return textureSampleCompareLevel(point_shadow_textures, point_shadow_textures_sampler, frag_ls, i32(light_id), depth);
#endif
}

fn fetch_directional_shadow(light_id: u32, frag_position: vec4<f32>, surface_normal: vec3<f32>) -> f32 {
    let light = lights.directional_lights[light_id];

    // The normal bias is scaled to the texel size.
    let normal_offset = light.shadow_normal_bias * surface_normal.xyz;
    let depth_offset = light.shadow_depth_bias * light.direction_to_light.xyz;
    let offset_position = vec4<f32>(frag_position.xyz + normal_offset + depth_offset, frag_position.w);

    let offset_position_clip = light.view_projection * offset_position;
    if (offset_position_clip.w <= 0.0) {
        return 1.0;
    }
    let offset_position_ndc = offset_position_clip.xyz / offset_position_clip.w;
    // No shadow outside the orthographic projection volume
    if (any(offset_position_ndc.xy < vec2<f32>(-1.0)) || offset_position_ndc.z < 0.0
            || any(offset_position_ndc > vec3<f32>(1.0))) {
        return 1.0;
    }

    // compute texture coordinates for shadow lookup, compensating for the Y-flip difference
    // between the NDC and texture coordinates
    let flip_correction = vec2<f32>(0.5, -0.5);
    let light_local = offset_position_ndc.xy * flip_correction + vec2<f32>(0.5, 0.5);

    let depth = offset_position_ndc.z;
    // do the lookup, using HW PCF and comparison
    // NOTE: Due to non-uniform control flow above, we must use the level variant of the texture
    // sampler to avoid use of implicit derivatives causing possible undefined behavior.
#ifdef NO_ARRAY_TEXTURES_SUPPORT
    return textureSampleCompareLevel(directional_shadow_textures, directional_shadow_textures_sampler, light_local, depth);
#else
    return textureSampleCompareLevel(directional_shadow_textures, directional_shadow_textures_sampler, light_local, i32(light_id), depth);
#endif
}

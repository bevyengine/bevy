#define_import_path bevy_pbr::shadows

#import bevy_pbr::{
    mesh_view_types::POINT_LIGHT_FLAGS_SPOT_LIGHT_Y_NEGATIVE,
    mesh_view_bindings as view_bindings,
    shadow_sampling::{SPOT_SHADOW_TEXEL_SIZE, sample_shadow_cubemap, sample_shadow_map}
}

#import bevy_render::{
    color_operations::hsv_to_rgb,
    maths::PI_2
}

const flip_z: vec3<f32> = vec3<f32>(1.0, 1.0, -1.0);

fn fetch_point_shadow(light_id: u32, frag_position: vec4<f32>, surface_normal: vec3<f32>) -> f32 {
    let light = &view_bindings::clusterable_objects.data[light_id];

    // because the shadow maps align with the axes and the frustum planes are at 45 degrees
    // we can get the worldspace depth by taking the largest absolute axis
    let surface_to_light = (*light).position_radius.xyz - frag_position.xyz;
    let surface_to_light_abs = abs(surface_to_light);
    let distance_to_light = max(surface_to_light_abs.x, max(surface_to_light_abs.y, surface_to_light_abs.z));

    // The normal bias here is already scaled by the texel size at 1 world unit from the light.
    // The texel size increases proportionally with distance from the light so multiplying by
    // distance to light scales the normal bias to the texel size at the fragment distance.
    let normal_offset = (*light).shadow_normal_bias * distance_to_light * surface_normal.xyz;
    let depth_offset = (*light).shadow_depth_bias * normalize(surface_to_light.xyz);
    let offset_position = frag_position.xyz + normal_offset + depth_offset;

    // similar largest-absolute-axis trick as above, but now with the offset fragment position
    let frag_ls = offset_position.xyz - (*light).position_radius.xyz ;
    let abs_position_ls = abs(frag_ls);
    let major_axis_magnitude = max(abs_position_ls.x, max(abs_position_ls.y, abs_position_ls.z));

    // NOTE: These simplifications come from multiplying:
    // projection * vec4(0, 0, -major_axis_magnitude, 1.0)
    // and keeping only the terms that have any impact on the depth.
    // Projection-agnostic approach:
    let zw = -major_axis_magnitude * (*light).light_custom_data.xy + (*light).light_custom_data.zw;
    let depth = zw.x / zw.y;

    // Do the lookup, using HW PCF and comparison. Cubemaps assume a left-handed coordinate space,
    // so we have to flip the z-axis when sampling.
    return sample_shadow_cubemap(frag_ls * flip_z, distance_to_light, depth, light_id);
}

fn fetch_spot_shadow(light_id: u32, frag_position: vec4<f32>, surface_normal: vec3<f32>) -> f32 {
    let light = &view_bindings::clusterable_objects.data[light_id];

    let surface_to_light = (*light).position_radius.xyz - frag_position.xyz;

    // construct the light view matrix
    var spot_dir = vec3<f32>((*light).light_custom_data.x, 0.0, (*light).light_custom_data.y);
    // reconstruct spot dir from x/z and y-direction flag
    spot_dir.y = sqrt(max(0.0, 1.0 - spot_dir.x * spot_dir.x - spot_dir.z * spot_dir.z));
    if (((*light).flags & POINT_LIGHT_FLAGS_SPOT_LIGHT_Y_NEGATIVE) != 0u) {
        spot_dir.y = -spot_dir.y;
    }

    // view matrix z_axis is the reverse of transform.forward()
    let fwd = -spot_dir;
    let distance_to_light = dot(fwd, surface_to_light);
    let offset_position =
        -surface_to_light
        + ((*light).shadow_depth_bias * normalize(surface_to_light))
        + (surface_normal.xyz * (*light).shadow_normal_bias) * distance_to_light;

    // the construction of the up and right vectors needs to precisely mirror the code
    // in render/light.rs:spot_light_view_matrix
    var sign = -1.0;
    if (fwd.z >= 0.0) {
        sign = 1.0;
    }
    let a = -1.0 / (fwd.z + sign);
    let b = fwd.x * fwd.y * a;
    let up_dir = vec3<f32>(1.0 + sign * fwd.x * fwd.x * a, sign * b, -sign * fwd.x);
    let right_dir = vec3<f32>(-b, -sign - fwd.y * fwd.y * a, fwd.y);
    let light_inv_rot = mat3x3<f32>(right_dir, up_dir, fwd);

    // because the matrix is a pure rotation matrix, the inverse is just the transpose, and to calculate
    // the product of the transpose with a vector we can just post-multiply instead of pre-multiplying.
    // this allows us to keep the matrix construction code identical between CPU and GPU.
    let projected_position = offset_position * light_inv_rot;

    // divide xy by perspective matrix "f" and by -projected.z (projected.z is -projection matrix's w)
    // to get ndc coordinates
    let f_div_minus_z = 1.0 / ((*light).spot_light_tan_angle * -projected_position.z);
    let shadow_xy_ndc = projected_position.xy * f_div_minus_z;
    // convert to uv coordinates
    let shadow_uv = shadow_xy_ndc * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);

    // 0.1 must match POINT_LIGHT_NEAR_Z
    let depth = 0.1 / -projected_position.z;

    return sample_shadow_map(
        shadow_uv,
        depth,
        i32(light_id) + view_bindings::lights.spot_light_shadowmap_offset,
        SPOT_SHADOW_TEXEL_SIZE
    );
}

fn get_cascade_index(light_id: u32, view_z: f32) -> u32 {
    let light = &view_bindings::lights.directional_lights[light_id];

    for (var i: u32 = 0u; i < (*light).num_cascades; i = i + 1u) {
        if (-view_z < (*light).cascades[i].far_bound) {
            return i;
        }
    }
    return (*light).num_cascades;
}

// Converts from world space to the uv position in the light's shadow map.
//
// The depth is stored in the return value's z coordinate. If the return value's
// w coordinate is 0.0, then we landed outside the shadow map entirely.
fn world_to_directional_light_local(
    light_id: u32,
    cascade_index: u32,
    offset_position: vec4<f32>
) -> vec4<f32> {
    let light = &view_bindings::lights.directional_lights[light_id];
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

fn sample_directional_cascade(light_id: u32, cascade_index: u32, frag_position: vec4<f32>, surface_normal: vec3<f32>) -> f32 {
    let light = &view_bindings::lights.directional_lights[light_id];
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
    return sample_shadow_map(light_local.xy, light_local.z, array_index, (*cascade).texel_size);
}

fn fetch_directional_shadow(light_id: u32, frag_position: vec4<f32>, surface_normal: vec3<f32>, view_z: f32) -> f32 {
    let light = &view_bindings::lights.directional_lights[light_id];
    let cascade_index = get_cascade_index(light_id, view_z);

    if (cascade_index >= (*light).num_cascades) {
        return 1.0;
    }

    var shadow = sample_directional_cascade(light_id, cascade_index, frag_position, surface_normal);

    // Blend with the next cascade, if there is one.
    let next_cascade_index = cascade_index + 1u;
    if (next_cascade_index < (*light).num_cascades) {
        let this_far_bound = (*light).cascades[cascade_index].far_bound;
        let next_near_bound = (1.0 - (*light).cascades_overlap_proportion) * this_far_bound;
        if (-view_z >= next_near_bound) {
            let next_shadow = sample_directional_cascade(light_id, next_cascade_index, frag_position, surface_normal);
            shadow = mix(shadow, next_shadow, (-view_z - next_near_bound) / (this_far_bound - next_near_bound));
        }
    }
    return shadow;
}

fn cascade_debug_visualization(
    output_color: vec3<f32>,
    light_id: u32,
    view_z: f32,
) -> vec3<f32> {
    let overlay_alpha = 0.95;
    let cascade_index = get_cascade_index(light_id, view_z);
    let cascade_color = hsv_to_rgb(
        f32(cascade_index) / f32(#{MAX_CASCADES_PER_LIGHT}u + 1u) * PI_2,
        1.0,
        0.5
    );
    return vec3<f32>(
        (1.0 - overlay_alpha) * output_color.rgb + overlay_alpha * cascade_color
    );
}

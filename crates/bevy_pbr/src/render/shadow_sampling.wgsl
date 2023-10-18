#define_import_path bevy_pbr::shadow_sampling

#import bevy_pbr::mesh_view_bindings as view_bindings
#import bevy_pbr::utils PI

// Do the lookup, using HW 2x2 PCF and comparison
fn sample_shadow_map_hardware(light_local: vec2<f32>, depth: f32, array_index: i32) -> f32 {
#ifdef NO_ARRAY_TEXTURES_SUPPORT
    return textureSampleCompare(
        view_bindings::directional_shadow_textures,
        view_bindings::directional_shadow_textures_sampler,
        light_local,
        depth,
    );
#else
    return textureSampleCompareLevel(
        view_bindings::directional_shadow_textures,
        view_bindings::directional_shadow_textures_sampler,
        light_local,
        array_index,
        depth,
    );
#endif
}

// https://web.archive.org/web/20230210095515/http://the-witness.net/news/2013/09/shadow-mapping-summary-part-1
fn sample_shadow_map_castano_thirteen(light_local: vec2<f32>, depth: f32, receiver_plane_depth_bias: vec2<f32>, array_index: i32) -> f32 {
    let shadow_map_size = vec2<f32>(textureDimensions(view_bindings::directional_shadow_textures));
    let inv_shadow_map_size = 1.0 / shadow_map_size;

    let uv = light_local * shadow_map_size;
    var base_uv = floor(uv + 0.5);
    let s = (uv.x + 0.5 - base_uv.x);
    let t = (uv.y + 0.5 - base_uv.y);
    base_uv -= 0.5;
    base_uv *= inv_shadow_map_size;

    let uw0 = (4.0 - 3.0 * s);
    let uw1 = 7.0;
    let uw2 = (1.0 + 3.0 * s);

    let u0 = (3.0 - 2.0 * s) / uw0 - 2.0;
    let u1 = (3.0 + s) / uw1;
    let u2 = s / uw2 + 2.0;

    let vw0 = (4.0 - 3.0 * t);
    let vw1 = 7.0;
    let vw2 = (1.0 + 3.0 * t);

    let v0 = (3.0 - 2.0 * t) / vw0 - 2.0;
    let v1 = (3.0 + t) / vw1;
    let v2 = t / vw2 + 2.0;

    var sum = 0.0;

    let sample_offset_u0_v0 = (vec2(u0, v0) * inv_shadow_map_size);
    sum += uw0 * vw0 * sample_shadow_map_hardware(base_uv + sample_offset_u0_v0, depth + dot(sample_offset_u0_v0, receiver_plane_depth_bias), array_index);
    let sample_offset_u1_v0 = (vec2(u1, v0) * inv_shadow_map_size);
    sum += uw1 * vw0 * sample_shadow_map_hardware(base_uv + sample_offset_u1_v0, depth + dot(sample_offset_u1_v0, receiver_plane_depth_bias), array_index);
    let sample_offset_u2_v0 = (vec2(u2, v0) * inv_shadow_map_size);
    sum += uw2 * vw0 * sample_shadow_map_hardware(base_uv + sample_offset_u2_v0, depth + dot(sample_offset_u2_v0, receiver_plane_depth_bias), array_index);

    let sample_offset_u0_v1 = (vec2(u0, v1) * inv_shadow_map_size);
    sum += uw0 * vw1 * sample_shadow_map_hardware(base_uv + sample_offset_u0_v1, depth + dot(sample_offset_u0_v1, receiver_plane_depth_bias), array_index);
    let sample_offset_u1_v1 = (vec2(u1, v1) * inv_shadow_map_size);
    sum += uw1 * vw1 * sample_shadow_map_hardware(base_uv + sample_offset_u1_v1, depth + dot(sample_offset_u1_v1, receiver_plane_depth_bias), array_index);
    let sample_offset_u2_v1 = (vec2(u2, v1) * inv_shadow_map_size);
    sum += uw2 * vw1 * sample_shadow_map_hardware(base_uv + sample_offset_u2_v1, depth + dot(sample_offset_u2_v1, receiver_plane_depth_bias), array_index);

    let sample_offset_u0_v2 = (vec2(u0, v2) * inv_shadow_map_size);
    sum += uw0 * vw2 * sample_shadow_map_hardware(base_uv + sample_offset_u0_v2, depth + dot(sample_offset_u0_v2, receiver_plane_depth_bias), array_index);
    let sample_offset_u1_v2 = (vec2(u1, v2) * inv_shadow_map_size);
    sum += uw1 * vw2 * sample_shadow_map_hardware(base_uv + sample_offset_u1_v2, depth + dot(sample_offset_u1_v2, receiver_plane_depth_bias), array_index);
    let sample_offset_u2_v2 = (vec2(u2, v2) * inv_shadow_map_size);
    sum += uw2 * vw2 * sample_shadow_map_hardware(base_uv + sample_offset_u2_v2, depth + dot(sample_offset_u2_v2, receiver_plane_depth_bias), array_index);

    return sum * (1.0 / 144.0);
}

// https://blog.demofox.org/2022/01/01/interleaved-gradient-noise-a-different-kind-of-low-discrepancy-sequence
fn interleaved_gradient_noise(pixel_coordinates: vec2<f32>) -> f32 {
    let frame = f32(view_bindings::globals.frame_count % 64u);
    let xy = pixel_coordinates + 5.588238 * frame;
    return fract(52.9829189 * fract(0.06711056 * xy.x + 0.00583715 * xy.y));
}

fn map(min1: f32, max1: f32, min2: f32, max2: f32, value: f32) -> f32 {
    return min2 + (value - min1) * (max2 - min2) / (max1 - min1);
}

fn sample_shadow_map_jimenez_fourteen(light_local: vec2<f32>, depth: f32, receiver_plane_depth_bias: vec2<f32>, array_index: i32, texel_size: f32) -> f32 {
    let shadow_map_size = vec2<f32>(textureDimensions(view_bindings::directional_shadow_textures));

    let random_angle = 2.0 * PI * interleaved_gradient_noise(light_local * shadow_map_size);
    let m = vec2(sin(random_angle), cos(random_angle));
    let rotation_matrix = mat2x2(
        m.y, -m.x,
        m.x, m.y
    );

    // Empirically chosen fudge factor to make PCF look better across different CSM cascades
    let f = map(0.00390625, 0.022949219, 0.015, 0.035, texel_size);
    let uv_offset_scale = f / (texel_size * shadow_map_size);

    // https://www.iryoku.com/next-generation-post-processing-in-call-of-duty-advanced-warfare (slides 120-135)
    let sample_offset1 = (rotation_matrix * vec2(-0.7071,  0.7071)) * uv_offset_scale;
    let sample_offset2 = (rotation_matrix * vec2(-0.0000, -0.8750)) * uv_offset_scale;
    let sample_offset3 = (rotation_matrix * vec2( 0.5303,  0.5303)) * uv_offset_scale;
    let sample_offset4 = (rotation_matrix * vec2(-0.6250, -0.0000)) * uv_offset_scale;
    let sample_offset5 = (rotation_matrix * vec2( 0.3536, -0.3536)) * uv_offset_scale;
    let sample_offset6 = (rotation_matrix * vec2(-0.0000,  0.3750)) * uv_offset_scale;
    let sample_offset7 = (rotation_matrix * vec2(-0.1768, -0.1768)) * uv_offset_scale;
    let sample_offset8 = (rotation_matrix * vec2( 0.1250,  0.0000)) * uv_offset_scale;

    var sum = 0.0;
    sum += sample_shadow_map_hardware(
        light_local + sample_offset1,
        depth + dot(sample_offset1, receiver_plane_depth_bias),
        array_index,
    );
    sum += sample_shadow_map_hardware(
        light_local + sample_offset2,
        depth + dot(sample_offset2, receiver_plane_depth_bias),
        array_index,
    );
    sum += sample_shadow_map_hardware(
        light_local + sample_offset3,
        depth + dot(sample_offset3, receiver_plane_depth_bias),
        array_index,
    );
    sum += sample_shadow_map_hardware(
        light_local + sample_offset4,
        depth + dot(sample_offset4, receiver_plane_depth_bias),
        array_index,
    );
    sum += sample_shadow_map_hardware(
        light_local + sample_offset5,
        depth + dot(sample_offset5, receiver_plane_depth_bias),
        array_index,
    );
    sum += sample_shadow_map_hardware(
        light_local + sample_offset6,
        depth + dot(sample_offset6, receiver_plane_depth_bias),
        array_index,
    );
    sum += sample_shadow_map_hardware(
        light_local + sample_offset7,
        depth + dot(sample_offset7, receiver_plane_depth_bias),
        array_index,
    );
    sum += sample_shadow_map_hardware(
        light_local + sample_offset8,
        depth + dot(sample_offset8, receiver_plane_depth_bias),
        array_index,
    );
    return sum / 8.0;
}

// Receiver Plane Depth Bias
// Isidoro06 - Slides 36-40 https://web.archive.org/web/20230309054654/http://developer.amd.com/wordpress/media/2012/10/Isidoro-ShadowMapping.pdf
// This implementation is from https://github.com/TheRealMJP/Shadows/blob/1a6d90c92ea58ccddb0dcd32b035c58b8f7784f4/Shadows/Mesh.hlsl#L102
fn compute_receiver_plane_depth_bias(tex_coord_dx: vec3<f32>, tex_coord_dy: vec3<f32>) -> vec2<f32> {
    var bias_uv: vec2<f32> = vec2<f32>(
        tex_coord_dy.y * tex_coord_dx.z - tex_coord_dx.y * tex_coord_dy.z,
        tex_coord_dx.x * tex_coord_dy.z - tex_coord_dy.x * tex_coord_dx.z
    );
    bias_uv = bias_uv * 1.0 / ((tex_coord_dx.x * tex_coord_dy.y) - (tex_coord_dx.y * tex_coord_dy.x));
    return bias_uv;
}

fn sample_shadow_map(light_local: vec2<f32>, depth: f32, array_index: i32, texel_size: f32) -> f32 {
#ifndef SHADOW_FILTER_METHOD_HARDWARE_2X2
    let shadow_pos = vec3(light_local, depth);
    let receiver_plane_depth_bias = compute_receiver_plane_depth_bias(
        dpdx(shadow_pos),
        dpdy(shadow_pos),
    );
#endif

#ifdef SHADOW_FILTER_METHOD_CASTANO_13
    return sample_shadow_map_castano_thirteen(
        light_local,
        depth,
        receiver_plane_depth_bias,
        array_index,
    );
#else ifdef SHADOW_FILTER_METHOD_JIMENEZ_14
    return sample_shadow_map_jimenez_fourteen(
        light_local,
        depth,
        receiver_plane_depth_bias,
        array_index,
        texel_size,
    );
#else ifdef SHADOW_FILTER_METHOD_HARDWARE_2X2
    return sample_shadow_map_hardware(light_local, depth, array_index);
#else
    // This needs a default return value to avoid shader compilation errors if it's compiled with no SHADOW_FILTER_METHOD_* defined.
    // (eg. if the normal prepass is enabled it ends up compiling this due to the normal prepass depending on pbr_functions, which depends on shadows)
    // This should never actually get used, as anyone using bevy's lighting/shadows should always have a SHADOW_FILTER_METHOD defined.
    // Set to 0 to make it obvious that something is wrong.
    return 0.0;
#endif
}

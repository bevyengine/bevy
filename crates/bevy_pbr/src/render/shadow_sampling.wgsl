#define_import_path bevy_pbr::shadow_sampling

#import bevy_pbr::mesh_view_bindings as view_bindings
#import bevy_pbr::utils PI

fn sample_shadow_map_hardware(light_local: vec2<f32>, depth: f32, array_index: i32) -> f32 {
    // Do the lookup, using HW 2x2 PCF and comparison
    // NOTE: Due to non-uniform control flow above, we must use the level variant of the texture
    // sampler to avoid use of implicit derivatives causing possible undefined behavior.
#ifdef NO_ARRAY_TEXTURES_SUPPORT
    return textureSampleCompareLevel(
        view_bindings::directional_shadow_textures,
        view_bindings::directional_shadow_textures_sampler,
        light_local,
        depth
    );
#else
    return textureSampleCompareLevel(
        view_bindings::directional_shadow_textures,
        view_bindings::directional_shadow_textures_sampler,
        light_local,
        array_index,
        depth
    );
#endif
}

// https://web.archive.org/web/20230210095515/http://the-witness.net/news/2013/09/shadow-mapping-summary-part-1
fn sample_shadow_map_castano_thirteen(light_local: vec2<f32>, depth: f32, array_index: i32) -> f32 {
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

    sum += uw0 * vw0 * sample_shadow_map_hardware(base_uv + (vec2(u0, v0) * inv_shadow_map_size), depth, array_index);
    sum += uw1 * vw0 * sample_shadow_map_hardware(base_uv + (vec2(u1, v0) * inv_shadow_map_size), depth, array_index);
    sum += uw2 * vw0 * sample_shadow_map_hardware(base_uv + (vec2(u2, v0) * inv_shadow_map_size), depth, array_index);

    sum += uw0 * vw1 * sample_shadow_map_hardware(base_uv + (vec2(u0, v1) * inv_shadow_map_size), depth, array_index);
    sum += uw1 * vw1 * sample_shadow_map_hardware(base_uv + (vec2(u1, v1) * inv_shadow_map_size), depth, array_index);
    sum += uw2 * vw1 * sample_shadow_map_hardware(base_uv + (vec2(u2, v1) * inv_shadow_map_size), depth, array_index);

    sum += uw0 * vw2 * sample_shadow_map_hardware(base_uv + (vec2(u0, v2) * inv_shadow_map_size), depth, array_index);
    sum += uw1 * vw2 * sample_shadow_map_hardware(base_uv + (vec2(u1, v2) * inv_shadow_map_size), depth, array_index);
    sum += uw2 * vw2 * sample_shadow_map_hardware(base_uv + (vec2(u2, v2) * inv_shadow_map_size), depth, array_index);

    return sum * (1.0 / 144.0);
}

// https://blog.demofox.org/2022/01/01/interleaved-gradient-noise-a-different-kind-of-low-discrepancy-sequence
fn interleaved_gradient_noise(pixel_coordinates: vec2<f32>) -> f32 {
    let frame = f32(view_bindings::globals.frame_count % 64u);
    let xy = pixel_coordinates + 5.588238 * frame;
    return fract(52.9829189 * fract(0.06711056 * xy.x + 0.00583715 * xy.y));
}

// https://www.iryoku.com/next-generation-post-processing-in-call-of-duty-advanced-warfare (slides 120-135)
const sample_offsets: array<vec2<f32>, 8> = array<vec2<f32>, 8>(
    vec2<f32>(-0.7071,  0.7071),
    vec2<f32>(-0.0000, -0.8750),
    vec2<f32>( 0.5303,  0.5303),
    vec2<f32>(-0.6250, -0.0000),
    vec2<f32>( 0.3536, -0.3536),
    vec2<f32>(-0.0000,  0.3750),
    vec2<f32>(-0.1768, -0.1768),
    vec2<f32>( 0.1250,  0.0000),
);
fn sample_shadow_map_jimenez_fourteen(light_local: vec2<f32>, depth: f32, array_index: i32) -> f32 {
    let shadow_map_size = vec2<f32>(textureDimensions(view_bindings::directional_shadow_textures));

    let random_angle = 2.0 * PI * interleaved_gradient_noise(light_local * shadow_map_size);
    let m = vec2(sin(random_angle), cos(random_angle));
    let rotation_matrix = mat2x2(
        m.y, -m.x,
        m.x, m.y
    );
    let scale = 4.0 / shadow_map_size;

    let sample_offset1 = (rotation_matrix * sample_offsets[0]) * scale;
    let sample_offset2 = (rotation_matrix * sample_offsets[1]) * scale;
    let sample_offset3 = (rotation_matrix * sample_offsets[2]) * scale;
    let sample_offset4 = (rotation_matrix * sample_offsets[3]) * scale;
    let sample_offset5 = (rotation_matrix * sample_offsets[4]) * scale;
    let sample_offset6 = (rotation_matrix * sample_offsets[5]) * scale;
    let sample_offset7 = (rotation_matrix * sample_offsets[6]) * scale;
    let sample_offset8 = (rotation_matrix * sample_offsets[7]) * scale;

    var sum = 0.0;
    sum += sample_shadow_map_hardware(light_local + sample_offset1, depth, array_index);
    sum += sample_shadow_map_hardware(light_local + sample_offset2, depth, array_index);
    sum += sample_shadow_map_hardware(light_local + sample_offset3, depth, array_index);
    sum += sample_shadow_map_hardware(light_local + sample_offset4, depth, array_index);
    sum += sample_shadow_map_hardware(light_local + sample_offset5, depth, array_index);
    sum += sample_shadow_map_hardware(light_local + sample_offset6, depth, array_index);
    sum += sample_shadow_map_hardware(light_local + sample_offset7, depth, array_index);
    sum += sample_shadow_map_hardware(light_local + sample_offset8, depth, array_index);
    return sum / 8.0;
}

fn sample_shadow_map(light_local: vec2<f32>, depth: f32, array_index: i32) -> f32 {
#ifdef SHADOW_FILTER_METHOD_CASTANO_13
    return sample_shadow_map_castano_thirteen(light_local, depth, array_index);
#else ifdef SHADOW_FILTER_METHOD_JIMENEZ_14
    return sample_shadow_map_jimenez_fourteen(light_local, depth, array_index);
#else
    return sample_shadow_map_hardware(light_local, depth, array_index);
#endif
}

fn sample_directional_cascade(light_id: u32, cascade_index: u32, frag_position: vec4<f32>, surface_normal: vec3<f32>) -> f32 {
    let light = &view_bindings::lights.directional_lights[light_id];
    let cascade = &(*light).cascades[cascade_index];

    // The normal bias is scaled to the texel size.
    let normal_offset = (*light).shadow_normal_bias * (*cascade).texel_size * surface_normal.xyz;
    let depth_offset = (*light).shadow_depth_bias * (*light).direction_to_light.xyz;
    let offset_position = vec4<f32>(frag_position.xyz + normal_offset + depth_offset, frag_position.w);

    let offset_position_clip = (*cascade).view_projection * offset_position;
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

    let array_index = i32((*light).depth_texture_base_index + cascade_index);
    return sample_shadow_map(light_local, depth, array_index);
}

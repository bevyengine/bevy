#define_import_path bevy_pbr::shadow_sampling

#import bevy_pbr::stochastic_sampling

// TODO: Allow user configuration
const STOCHASTIC_PCF_SAMPLES = 3u;
const STOCHASTIC_PCF_RADIUS = 2u;

fn sample_cascade_simple(light_local: vec2<f32>, depth: f32, array_index: i32) -> f32 {
    // Do the lookup, using HW PCF and comparison
    // NOTE: Due to non-uniform control flow above, we must use the level variant of the texture
    // sampler to avoid use of implicit derivatives causing possible undefined behavior.
#ifdef NO_ARRAY_TEXTURES_SUPPORT
    return textureSampleCompareLevel(
        directional_shadow_textures,
        directional_shadow_textures_sampler,
        light_local,
        depth
    );
#else
    return textureSampleCompareLevel(
        directional_shadow_textures,
        directional_shadow_textures_sampler,
        light_local,
        array_index,
        depth
    );
#endif
}

// https://web.archive.org/web/20230210095515/http://the-witness.net/news/2013/09/shadow-mapping-summary-part-1
fn sample_cascade_the_witness(light_local: vec2<f32>, depth: f32, array_index: i32) -> f32 {
    // TODO: Configurable filter radius (currently 5x5)

    let cascade_size = vec2<f32>(textureDimensions(directional_shadow_textures));
    let inv_cascade_size = 1.0 / cascade_size;

    let uv = light_local * cascade_size;
    var base_uv = floor(uv + 0.5);
    let s = (uv.x + 0.5 - base_uv.x);
    let t = (uv.y + 0.5 - base_uv.y);
    base_uv -= 0.5;
    base_uv *= inv_cascade_size;

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

    sum += uw0 * vw0 * sample_cascade_simple(base_uv + (vec2(u0, v0) * inv_cascade_size), depth, array_index);
    sum += uw1 * vw0 * sample_cascade_simple(base_uv + (vec2(u1, v0) * inv_cascade_size), depth, array_index);
    sum += uw2 * vw0 * sample_cascade_simple(base_uv + (vec2(u2, v0) * inv_cascade_size), depth, array_index);

    sum += uw0 * vw1 * sample_cascade_simple(base_uv + (vec2(u0, v1) * inv_cascade_size), depth, array_index);
    sum += uw1 * vw1 * sample_cascade_simple(base_uv + (vec2(u1, v1) * inv_cascade_size), depth, array_index);
    sum += uw2 * vw1 * sample_cascade_simple(base_uv + (vec2(u2, v1) * inv_cascade_size), depth, array_index);

    sum += uw0 * vw2 * sample_cascade_simple(base_uv + (vec2(u0, v2) * inv_cascade_size), depth, array_index);
    sum += uw1 * vw2 * sample_cascade_simple(base_uv + (vec2(u1, v2) * inv_cascade_size), depth, array_index);
    sum += uw2 * vw2 * sample_cascade_simple(base_uv + (vec2(u2, v2) * inv_cascade_size), depth, array_index);

    return sum / 144.0;
}

fn sample_cascade_stochastic(light_local: vec2<f32>, depth: f32, array_index: i32) -> f32 {
    var sum = 0.0;
    let cascade_size = textureDimensions(directional_shadow_textures);
    for (var sample_i = 0u; sample_i < STOCHASTIC_PCF_SAMPLES; sample_i += 1u) {
        let sample_uv = stochastic_uv(light_local, sample_i, f32(STOCHASTIC_PCF_RADIUS), cascade_size);

        sum += sample_cascade_simple(sample_uv, depth, array_index);
    }
    return sum / f32(STOCHASTIC_PCF_SAMPLES);
}

fn sample_cascade(light_id: u32, cascade_index: u32, frag_position: vec4<f32>, surface_normal: vec3<f32>) -> f32 {
    let light = &lights.directional_lights[light_id];
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
#ifdef STOCHASTIC_SAMPLING
    return sample_cascade_stochastic(light_local, depth, array_index);
#else
    return sample_cascade_the_witness(light_local, depth, array_index);
#endif // STOCHASTIC_SAMPLING
}

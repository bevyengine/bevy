#define_import_path bevy_pbr::lightmap

#import bevy_pbr::mesh_bindings::mesh

#ifdef MULTIPLE_LIGHTMAPS_IN_ARRAY
@group(2) @binding(4) var lightmaps_textures: binding_array<texture_2d<f32>, 4>;
@group(2) @binding(5) var lightmaps_samplers: binding_array<sampler, 4>;
#else   // MULTIPLE_LIGHTMAPS_IN_ARRAY
@group(2) @binding(4) var lightmaps_texture: texture_2d<f32>;
@group(2) @binding(5) var lightmaps_sampler: sampler;
#endif  // MULTIPLE_LIGHTMAPS_IN_ARRAY

// Samples the lightmap, if any, and returns indirect illumination from it.
fn lightmap(uv: vec2<f32>, exposure: f32, instance_index: u32) -> vec3<f32> {
    let packed_uv_rect = mesh[instance_index].lightmap_uv_rect;
    let uv_rect = vec4<f32>(
        unpack2x16unorm(packed_uv_rect.x),
        unpack2x16unorm(packed_uv_rect.y),
    );
    let lightmap_uv = mix(uv_rect.xy, uv_rect.zw, uv);
    let lightmap_slot = mesh[instance_index].material_and_lightmap_bind_group_slot >> 16u;

    // Bicubic 4-tap
    // https://developer.nvidia.com/gpugems/gpugems2/part-iii-high-quality-rendering/chapter-20-fast-third-order-texture-filtering
    // https://advances.realtimerendering.com/s2021/jpatry_advances2021/index.html#/111/0/2
#ifdef LIGHTMAP_BICUBIC_SAMPLING
    let texture_size = vec2<f32>(lightmap_size(lightmap_slot));
    let texel_size = 1.0 / texture_size;
    let puv = lightmap_uv * texture_size + 0.5;
    let iuv = floor(puv);
    let fuv = fract(puv);
    let g0x = g0(fuv.x);
    let g1x = g1(fuv.x);
    let h0x = h0_approx(fuv.x);
    let h1x = h1_approx(fuv.x);
    let h0y = h0_approx(fuv.y);
    let h1y = h1_approx(fuv.y);
    let p0 = (vec2(iuv.x + h0x, iuv.y + h0y) - 0.5) * texel_size;
    let p1 = (vec2(iuv.x + h1x, iuv.y + h0y) - 0.5) * texel_size;
    let p2 = (vec2(iuv.x + h0x, iuv.y + h1y) - 0.5) * texel_size;
    let p3 = (vec2(iuv.x + h1x, iuv.y + h1y) - 0.5) * texel_size;
    let color = g0(fuv.y) * (g0x * sample(p0, lightmap_slot) + g1x * sample(p1, lightmap_slot)) + g1(fuv.y) * (g0x * sample(p2, lightmap_slot) + g1x * sample(p3, lightmap_slot));
#else
    let color = sample(lightmap_uv, lightmap_slot);
#endif

    return color * exposure;
}

fn lightmap_size(lightmap_slot: u32) -> vec2<u32> {
#ifdef MULTIPLE_LIGHTMAPS_IN_ARRAY
    return textureDimensions(lightmaps_textures[lightmap_slot]);
#else
    return textureDimensions(lightmaps_texture);
#endif
}

fn sample(uv: vec2<f32>, lightmap_slot: u32) -> vec3<f32> {
    // Mipmapping lightmaps is usually a bad idea due to leaking across UV
    // islands, so there's no harm in using mip level 0 and it lets us avoid
    // control flow uniformity problems.
#ifdef MULTIPLE_LIGHTMAPS_IN_ARRAY
    return textureSampleLevel(lightmaps_textures[lightmap_slot], lightmaps_samplers[lightmap_slot], uv, 0.0).rgb;
#else
    return textureSampleLevel(lightmaps_texture, lightmaps_sampler, uv, 0.0).rgb;
#endif
}

fn w0(a: f32) -> f32 {
    return (1.0 / 6.0) * (a * (a * (-a + 3.0) - 3.0) + 1.0);
}

fn w1(a: f32) -> f32 {
    return (1.0 / 6.0) * (a * a * (3.0 * a - 6.0) + 4.0);
}

fn w2(a: f32) -> f32 {
    return (1.0 / 6.0) * (a * (a * (-3.0 * a + 3.0) + 3.0) + 1.0);
}

fn w3(a: f32) -> f32 {
    return (1.0 / 6.0) * (a * a * a);
}

fn g0(a: f32) -> f32 {
    return w0(a) + w1(a);
}

fn g1(a: f32) -> f32 {
    return w2(a) + w3(a);
}

fn h0_approx(a: f32) -> f32 {
    return -0.2 - a * (0.24 * a - 0.44);
}

fn h1_approx(a: f32) -> f32 {
    return 1.0 + a * (0.24 * a - 0.04);
}

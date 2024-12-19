#define_import_path bevy_pbr::lightmap

#import bevy_pbr::mesh_bindings::mesh

#ifdef MULTIPLE_LIGHTMAPS_IN_ARRAY
@group(1) @binding(4) var lightmaps_textures: binding_array<texture_2d<f32>, 4>;
@group(1) @binding(5) var lightmaps_samplers: binding_array<sampler, 4>;
#else   // MULTIPLE_LIGHTMAPS_IN_ARRAY
@group(1) @binding(4) var lightmaps_texture: texture_2d<f32>;
@group(1) @binding(5) var lightmaps_sampler: sampler;
#endif  // MULTIPLE_LIGHTMAPS_IN_ARRAY

// Samples the lightmap, if any, and returns indirect illumination from it.
fn lightmap(uv: vec2<f32>, exposure: f32, instance_index: u32) -> vec3<f32> {
    let packed_uv_rect = mesh[instance_index].lightmap_uv_rect;
    let uv_rect = vec4<f32>(vec4<u32>(
        packed_uv_rect.x & 0xffffu,
        packed_uv_rect.x >> 16u,
        packed_uv_rect.y & 0xffffu,
        packed_uv_rect.y >> 16u)) / 65535.0;

    let lightmap_uv = mix(uv_rect.xy, uv_rect.zw, uv);

    // Mipmapping lightmaps is usually a bad idea due to leaking across UV
    // islands, so there's no harm in using mip level 0 and it lets us avoid
    // control flow uniformity problems.
    //
    // TODO(pcwalton): Consider bicubic filtering.
#ifdef MULTIPLE_LIGHTMAPS_IN_ARRAY
    let lightmap_slot = mesh[instance_index].material_and_lightmap_bind_group_slot >> 16u;
    return textureSampleLevel(
        lightmaps_textures[lightmap_slot],
        lightmaps_samplers[lightmap_slot],
        lightmap_uv,
        0.0
    ).rgb * exposure;
#else   // MULTIPLE_LIGHTMAPS_IN_ARRAY
    return textureSampleLevel(
        lightmaps_texture,
        lightmaps_sampler,
        lightmap_uv,
        0.0
    ).rgb * exposure;
#endif  // MULTIPLE_LIGHTMAPS_IN_ARRAY
}

#define_import_path bevy_pbr::lightmaps

#import bevy_pbr::mesh_bindings mesh

struct Lightmap {
    uv_rect: vec4<f32>,
    texture_array_index: u32,
};

struct Lightmaps {
    data: array<Lightmap, 1024u>,
};

#ifdef MESH_BINDGROUP_1
@group(1) @binding(4) var lightmaps_texture: texture_2d_array<f32>;
@group(1) @binding(5) var lightmaps_sampler: sampler;
@group(1) @binding(6) var<uniform> lightmaps: Lightmaps;
#else
@group(2) @binding(4) var lightmaps_texture: texture_2d_array<f32>;
@group(2) @binding(5) var lightmaps_sampler: sampler;
@group(2) @binding(6) var<uniform> lightmaps: Lightmaps;
#endif  // MESH_BINDGROUP_1

// Samples the lightmap, if any, and returns indirect illumination from it.
fn lightmap(uv: vec2<f32>, instance_index: u32) -> vec3<f32> {
    // Check to see if we have a lightmap at all.
    let lightmap_index = mesh[instance_index].lightmap_index;
    if (lightmap_index == 0xffffffffu) {
        return vec3(0.0);
    }

    // If we don't have a lightmap, then we still might have an invalid texture
    // array index. Check for that.
    let texture_array_index = lightmaps.data[lightmap_index].texture_array_index;
    if (texture_array_index == 0xffffffffu) {
        return vec3(0.0);
    }

    // Calculate the lightmap UV.
    let uv_rect = lightmaps.data[lightmap_index].uv_rect;
    let lightmap_uv = mix(uv_rect.xy, uv_rect.zw, uv);

    // Mipmapping lightmaps is usually a bad idea due to leaking across UV
    // islands, so there's no harm in using mip level 0 and it lets us avoid
    // control flow uniformity problems.
    //
    // TODO(pcwalton): Consider bicubic filtering.
    return textureSampleLevel(lightmaps_texture, lightmaps_sampler, lightmap_uv, texture_array_index, 0.0).rgb;
}

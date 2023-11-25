#define_import_path bevy_pbr::lightmaps

#import bevy_pbr::mesh_bindings mesh

struct Lightmap {
    exposure: f32,
};

#ifdef MESH_BINDGROUP_1
@group(1) @binding(4) var lightmaps_texture: texture_2d<f32>;
@group(1) @binding(5) var lightmaps_sampler: sampler;
@group(1) @binding(6) var<uniform> lightmap_uniform: Lightmap;
#else
@group(2) @binding(4) var lightmaps_texture: texture_2d<f32>;
@group(2) @binding(5) var lightmaps_sampler: sampler;
@group(2) @binding(6) var<uniform> lightmap_uniform: Lightmap;
#endif  // MESH_BINDGROUP_1

// Samples the lightmap, if any, and returns indirect illumination from it.
fn lightmap(uv: vec2<f32>, instance_index: u32) -> vec3<f32> {
    // Calculate the lightmap UV.
    let uv_rect = mesh[instance_index].lightmap_uv_rect;
    if (all(uv_rect == vec4(0.0))) {
        return vec3(0.0);
    }

    let lightmap_uv = mix(uv_rect.xy, uv_rect.zw, uv);

    // Mipmapping lightmaps is usually a bad idea due to leaking across UV
    // islands, so there's no harm in using mip level 0 and it lets us avoid
    // control flow uniformity problems.
    //
    // TODO(pcwalton): Consider bicubic filtering.
    return textureSampleLevel(
        lightmaps_texture,
        lightmaps_sampler,
        lightmap_uv,
        0.0).rgb * lightmap_uniform.exposure;
}

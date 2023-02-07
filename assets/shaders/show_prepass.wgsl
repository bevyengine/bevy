#import bevy_pbr::mesh_types
#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::prepass_utils

struct ShowPrepassSettings {
    show_depth: f32,
    show_normals: f32,
    is_webgl: f32,
    padding__: f32,
}
@group(1) @binding(0)
var<uniform> settings: ShowPrepassSettings;
@group(1) @binding(1)
var show_prepass_sampler: sampler;

@fragment
fn fragment(
    @builtin(position) frag_coord: vec4<f32>,
    #ifndef WEBGL
    @builtin(sample_index) sample_index: u32,
    #endif// WEBGL
    #import bevy_pbr::mesh_vertex_output
) -> @location(0) vec4<f32> {
    if settings.show_depth == 1.0 {
        #ifdef WEBGL
        // prepass_depth() uses textureLoad which doesn't work in WebGL for depth textures.
        // Instead we need to use a sampler
        let dims = textureDimensions(depth_prepass_texture);
        let uv = frag_coord.xy / vec2<f32>(dims);
        let depth = textureSample(depth_prepass_texture, show_prepass_sampler, uv);
        #else
        let depth = prepass_depth(frag_coord, sample_index);
        #endif // WEBGL
        return vec4(depth, depth, depth, 1.0);
    } else if settings.show_normals == 1.0 {
        #ifdef WEBGL
        let normal = prepass_normal(frag_coord, 0u);
        #else // WEBGL
        let normal = prepass_normal(frag_coord, sample_index);
        #endif // WEBGL
        return vec4(normal, 1.0);
    }

    return vec4(0.0);
}

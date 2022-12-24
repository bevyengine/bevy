@group(1) @binding(0)
var textures: binding_array<texture_2d<f32>>;
@group(1) @binding(1)
var samplers: binding_array<sampler>;

@fragment
fn fragment(
    #import bevy_pbr::mesh_vertex_output
) -> @location(0) vec4<f32> {
#ifdef VERTEX_UVS
    // Select the texture to sample from using non-uniform uv coordinates
    let coords = clamp(vec2<u32>(uv * 4.0), vec2<u32>(0u), vec2<u32>(3u));
    let index = coords.y * 4u + coords.x;
    return textureSample(textures[index], samplers[index], uv * 3.9 + 0.05);
#else
    return vec4<f32>(1.0, 0.0, 1.0, 1.0);
#endif
}

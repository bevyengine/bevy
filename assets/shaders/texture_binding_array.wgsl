@group(1) @binding(0)
var textures: binding_array<texture_2d<f32>>;
@group(1) @binding(1)
var nearest_sampler: sampler;
// We can also have array of samplers
// var samplers: binding_array<sampler>;

@fragment
fn fragment(
    #import bevy_pbr::mesh_vertex_output
) -> @location(0) vec4<f32> {
    // Select the texture to sample from using non-uniform uv coordinates
    let coords = clamp(vec2<u32>(uv * 4.0), vec2<u32>(0u), vec2<u32>(3u));
    let index = coords.y * 4u + coords.x;
    let inner_uv = fract(uv * 4.0);
    return textureSample(textures[index], nearest_sampler, inner_uv);
}

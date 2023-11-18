#import bevy_render::view::{View, coords_to_ray_direction}

@group(0) @binding(0) var skybox: texture_cube<f32>;
@group(0) @binding(1) var skybox_sampler: sampler;
@group(0) @binding(2) var<uniform> view: View;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

//  3 |  2.
//  2 |  :  `.
//  1 |  x-----x.
//  0 |  |  s  |  `.
// -1 |  0-----x.....1
//    +---------------
//      -1  0  1  2  3
//
// The axes are clip-space x and y. The region marked s is the visible region.
// The digits in the corners of the right-angled triangle are the vertex
// indices.
@vertex
fn skybox_vertex(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // See the explanation above for how this works.
    let clip_position = vec4(
        f32(vertex_index & 1u),
        f32((vertex_index >> 1u) & 1u),
        0.25,
        0.5
    ) * 4.0 - vec4(1.0);

    return VertexOutput(clip_position);
}

@fragment
fn skybox_fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let ray_direction = coords_to_ray_direction(in.position.xy, view);

    // Cube maps are left-handed so we negate the z coordinate.
    return textureSample(skybox, skybox_sampler, ray_direction * vec3(1.0, 1.0, -1.0));
}

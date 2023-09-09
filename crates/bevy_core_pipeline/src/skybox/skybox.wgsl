#import bevy_render::view View

@group(0) @binding(0)
var skybox: texture_cube<f32>;
@group(0) @binding(1)
var skybox_sampler: sampler;
@group(0) @binding(2)
var<uniform> view: View;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
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
    // Use the position on the near clipping plane to avoid -inf world position
    // because the far plane of an infinite reverse projection is at infinity.
    // NOTE: The clip position has a w component equal to 1.0 so we don't need
    // to apply a perspective divide to it before inverse-projecting it.
    let world_position_homogeneous = view.inverse_view_proj * vec4(clip_position.xy, 1.0, 1.0);
    let world_position = world_position_homogeneous.xyz / world_position_homogeneous.w;

    return VertexOutput(clip_position, world_position);
}

@fragment
fn skybox_fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // The skybox cubemap is sampled along the direction from the camera world
    // position, to the fragment world position on the near clipping plane
    let ray_direction = in.world_position - view.world_position;
    // cube maps are left-handed so we negate the z coordinate
    return textureSample(skybox, skybox_sampler, ray_direction * vec3(1.0, 1.0, -1.0));
}

#import bevy_pbr::mesh_view_bindings

@group(1) @binding(0)
var texture: texture_2d<f32>;

@group(1) @binding(1)
var our_sampler: sampler;

@group(1) @binding(2)
var<uniform> offset_r: vec2<f32>;
@group(1) @binding(3)
var<uniform> offset_g: vec2<f32>;
@group(1) @binding(4)
var<uniform> offset_b: vec2<f32>;

@fragment
fn fragment(
    @builtin(position) position: vec4<f32>,
    #import bevy_sprite::mesh2d_vertex_output
) -> @location(0) vec4<f32> {
    // Get screen position with coordinates from 0 to 1
    let uv = position.xy / vec2<f32>(view.width, view.height);

    // Sample each color channel with an arbitrary shift
    var output_color = vec4<f32>(
        textureSample(texture, our_sampler, uv + offset_r).r,
        textureSample(texture, our_sampler, uv + offset_g).g,
        textureSample(texture, our_sampler, uv + offset_b).b,
        1.0
    );

    return output_color;
}

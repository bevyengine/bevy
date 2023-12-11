// This shader draws a circle with a given input color
#import bevy_sprite::sprite_vertex_output::SpriteVertexOutput

// struct CustomSpriteMaterial {
//     @location(0) color: vec4<f32>
// }

@group(1) @binding(0) var sprite_texture: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;

// @group(2) @binding(0)
// var<uniform> input: CustomSpriteMaterial;

@fragment
fn fragment(in: SpriteVertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(sprite_texture, sprite_sampler, in.uv);
    // let g = input.color.r * color.r + input.color.g * color.g + input.color.b * color.b;
    let g = 0.21 * color.r + 0.72 * color.g + 0.07 * color.b;
    return vec4<f32>(g, g, g, color.a);
}

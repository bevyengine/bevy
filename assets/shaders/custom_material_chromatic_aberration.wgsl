#import bevy_pbr::mesh_view_bind_group

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
};

struct CustomMaterial {
    mouse_position: f32;
};

[[group(1), binding(0)]]
var texture: texture_2d<f32>;

[[group(1), binding(1)]]
var our_sampler: sampler;


[[stage(fragment)]]
fn fragment(input: VertexOutput) -> [[location(0)]] vec4<f32> {
    let uv = input.clip_position.xy / vec2<f32>(view.width, view.height);
    var output_color = vec4<f32>(
        textureSample(texture, our_sampler, uv + vec2<f32>(0.01, -0.01)).r,
        textureSample(texture, our_sampler, uv + vec2<f32>(-0.02, 0.0)).g,
        textureSample(texture, our_sampler, uv + vec2<f32>(-0.01, 0.03)).b,
        1.0
        );

    //var output_color = textureSample(texture, our_sampler, uv.xy);

    return output_color;
}
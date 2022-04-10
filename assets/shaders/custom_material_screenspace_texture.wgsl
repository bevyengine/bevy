#import bevy_pbr::mesh_view_bind_group

[[group(1), binding(0)]]
var texture: texture_2d<f32>;
[[group(1), binding(1)]]
var texture_sampler: sampler;

[[stage(fragment)]]
fn fragment([[builtin(position)]] position: vec4<f32>) -> [[location(0)]] vec4<f32> {
    let uv = position.xy / vec2<f32>(view.width, view.height);
    let color = textureSample(texture, texture_sampler, uv);
    return color;
}

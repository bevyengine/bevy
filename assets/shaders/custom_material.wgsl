//[[block]] // someone said that it is deprecated
struct CustomMaterial {
    color: vec4<f32>;
};
[[group(1), binding(0)]]
var<uniform> material: CustomMaterial;

[[stage(fragment)]]
fn fragment() -> [[location(0)]] vec4<f32> {
    return material.color;
}

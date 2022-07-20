struct CustomMaterial {
    color: vec4<f32>;
};

[[group(1), binding(0)]]
var<uniform> material: CustomMaterial;

[[stage(fragment)]]
fn fragment() -> [[location(0)]] vec4<f32> {
#ifdef IS_RED
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
#else
    return material.color;
#endif
}

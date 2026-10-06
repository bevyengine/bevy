import package::mesh2d::view_bindings::lights_2d;

fn apply_lighting_2d(color: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(color.rgb * lights_2d.ambient, color.a);
}

import package::mesh2d::view_bindings::lights_2d;

fn apply_lighting_2d(color: vec4<f32>, world_position: vec2<f32>) -> vec4<f32> {
    var light = lights_2d.ambient;

    for (var i = 0u; i < lights_2d.point_light_count; i++) {
        let point_light = lights_2d.point_lights[i];
        let dist = distance(world_position, point_light.position);
        if dist < point_light.range {
            light += point_light.color * attenuation(dist, point_light.range, point_light.intensity, point_light.falloff);
        }
    }

    return vec4<f32>(color.rgb * light, color.a);
}

// See https://lisyarus.github.io/blog/posts/point-light-attenuation.html
fn attenuation(dist: f32, range: f32, intensity: f32, falloff: f32) -> f32 {
    let s = dist / range;
    if s > 1.0 {
        return 0.0;
    }
    let s2 = s * s;
    let window = 1.0 - s2;
    return intensity * window * window / (1.0 + falloff * s2);
}

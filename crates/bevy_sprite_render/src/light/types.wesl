struct PointLight2d {
    position: vec2<f32>,
    range: f32,
    intensity: f32,
    color: vec4<f32>,
}

struct Lights2d {
    ambient: vec3<f32>,
    point_light_count: u32,
    point_lights: array<PointLight2d, u32(constants::MAX_POINT_LIGHTS_2D)>,
}

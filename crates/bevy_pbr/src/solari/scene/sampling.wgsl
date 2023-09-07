fn rand_u(state: ptr<function, u32>) -> u32 {
    *state = *state * 747796405u + 2891336453u;
    let word = ((*state >> ((*state >> 28u) + 4u)) ^ *state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn rand_f(state: ptr<function, u32>) -> f32 {
    *state = *state * 747796405u + 2891336453u;
    let word = ((*state >> ((*state >> 28u) + 4u)) ^ *state) * 277803737u;
    return f32((word >> 22u) ^ word) * bitcast<f32>(0x2f800004u);
}

fn rand_vec2(state: ptr<function, u32>) -> vec2<f32> {
    return vec2(rand_f(state), rand_f(state));
}

fn rand_range_u(n: u32, state: ptr<function, u32>) -> u32 {
    return rand_u(state) % n;
}

fn sample_cosine_hemisphere(normal: vec3<f32>, state: ptr<function, u32>) -> vec3<f32> {
    let cos_theta = 2.0 * rand_f(state) - 1.0;
    let phi = 2.0 * PI * rand_f(state);
    let sin_theta = sqrt(max(1.0 - cos_theta * cos_theta, 0.0));
    let sin_phi = sin(phi);
    let cos_phi = cos(phi);
    let unit_sphere_direction = normalize(vec3(sin_theta * cos_phi, cos_theta, sin_theta * sin_phi));
    return normal + unit_sphere_direction;
}

fn make_polka_dots(pos: vec2<f32>, time: f32) -> vec4<f32> {
    let scaled_pos = pos * 6.0;
    let cell = vec2<f32>(fract(scaled_pos.x), fract(scaled_pos.y));
    var dist_from_center = distance(cell, vec2<f32>(0.5));

    let is_even = (floor(scaled_pos.x) + floor(scaled_pos.y)) % 2.0;

    var dot_color = vec3<f32>(0.0);
    var is_dot = 0.0;

    @if(!PARTY_MODE) {
        let color1 = vec3<f32>(1.0, 0.4, 0.8);  // pink
        let color2 = vec3<f32>(0.6, 0.2, 1.0);  // purple
        dot_color = mix(color1, color2, is_even);
        is_dot = step(dist_from_center, 0.3);
    } @else {
        let grid_x = floor(scaled_pos.x);
        let grid_y = floor(scaled_pos.y);
        let wave_speed = 3.0;
        let wave_phase = time * wave_speed;

        let diagonal_pos = (grid_x + grid_y) * 0.5;
        let wave_value = sin(diagonal_pos + wave_phase);

        let wave_normalized = (wave_value + 1.0) * 0.5;

        let color1 = vec3<f32>(1.0, 0.3, 0.7);
        let color2 = vec3<f32>(0.5, 0.1, 1.0);
        let intense_color1 = vec3<f32>(1.0, 0.1, 0.9);
        let intense_color2 = vec3<f32>(0.8, 0.0, 1.0);

        let animated_color1 = mix(color1, intense_color1, wave_normalized);
        let animated_color2 = mix(color2, intense_color2, wave_normalized);

        dot_color = mix(animated_color1, animated_color2, is_even);

        let size_mod = 0.15 * wave_value;
        dist_from_center = dist_from_center * (1.0 - size_mod);
        // Animate whether something is a dot by position but also time
        is_dot = step(dist_from_center, 0.3 + wave_normalized * 0.2);
    }

    return vec4<f32>(dot_color * is_dot, 1.0);
}

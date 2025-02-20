fn make_polka_dots(pos: vec2<f32>, time: f32) -> vec4<f32> {
    // Create repeating circles
    let scaled_pos = pos * 6.0;
    let cell = vec2<f32>(fract(scaled_pos.x), fract(scaled_pos.y));
    let dist_from_center = distance(cell, vec2<f32>(0.5));
    
    // Make dots alternate between pink and purple
    let is_even = (floor(scaled_pos.x) + floor(scaled_pos.y)) % 2.0;

    var dot_color = vec3<f32>(0.0);
    @if(!PARTY_MODE) {
        let color1 = vec3<f32>(1.0, 0.4, 0.8);  // pink
        let color2 = vec3<f32>(0.6, 0.2, 1.0);  // purple
        dot_color = mix(color1, color2, is_even);
    }
    // Animate the colors in party mode
    @if(PARTY_MODE) {
        let color1 = vec3<f32>(1.0, 0.2, 0.2);  // red
        let color2 = vec3<f32>(0.2, 0.2, 1.0);  // blue
        let oscillation = (sin(time * 10.0) + 1.0) * 0.5;
        let animated_color1 = mix(color1, color2, oscillation);
        let animated_color2 = mix(color2, color1, oscillation);
        dot_color = mix(animated_color1, animated_color2, is_even);
    }

    // Draw the dot
    let is_dot = step(dist_from_center, 0.3);
    return vec4<f32>(dot_color * is_dot, is_dot);
}
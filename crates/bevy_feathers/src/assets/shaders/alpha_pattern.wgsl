// This shader draws a checkerboard pattern
#import bevy_ui::ui_vertex_output::UiVertexOutput
#import bevy_ui::ui_node::sd_rounded_box;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let uv = (in.uv - vec2<f32>(0.5, 0.5)) * in.size / 16.;
    let check = select(0.0, 1.0, (fract(uv.x) < 0.5) != (fract(uv.y) < 0.5));
    let bg = mix(vec3<f32>(0.2, 0.2, 0.2), vec3<f32>(0.6, 0.6, 0.6), check);

    let size = vec2<f32>(in.size.x, in.size.y);
    // With rounded non-elliptical border radius, in.border_radius_x == in.border_radius.y. 
    let external_distance = sd_rounded_box((in.uv - 0.5) * size, size, in.border_radius_x, in.border_radius_y);
    let alpha = smoothstep(0.5, -0.5, external_distance);

    return vec4<f32>(bg, alpha);
}
// This shader draws a hue ring and an inner whiteness/blackness triangle.
#import bevy_ui::ui_vertex_output::UiVertexOutput
#import bevy_render::color_operations::{
    hwb_to_linear_rgb,
}
#import bevy_render::maths::PI_2

/// Constants must be the same as in `color_triangle.rs`
const RING_WIDTH: f32 = 12.0;
const SPACING: f32 = 4.0;
const MIN_HEIGHT: f32 = 100.0;
const PADDING: f32 = 4.0;
const MIN_DIAMETER: f32 = MIN_HEIGHT - 2.0 * PADDING;

struct ColorPlaneUniform {
  // Hue in degrees, in the range [0, 360).
  hue : f32,
#ifdef SIXTEEN_BYTE_ALIGNMENT
  _webgl2_padding_12b : vec3<f32>,
#endif
}

@group(1) @binding(0) var<uniform> uniform_data : ColorPlaneUniform;

// 2d cross product
fn cross_2d(a: vec2<f32>, b: vec2<f32>) -> f32 {
    return a.x * b.y - a.y * b.x;
}

// Line distance for triangle SDF, positive on left
fn line_distance(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
    return cross_2d(b - a, p - a) / distance(a, b);
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let min_side = max(min(in.size.x, in.size.y), MIN_DIAMETER);
    let diameters = min_side / in.size;
    // UV space is from -0.5 to 0.5
    let centered = (in.uv - vec2(0.5)) / diameters;
    let radial = length(centered);
    let aa = fwidth(radial);

    let inner_radius = 0.5 - RING_WIDTH / min_side;
    let ring_hue = fract(atan2(centered.y, centered.x) / PI_2);
    let ring_alpha = smoothstep(inner_radius - aa, inner_radius, radial)
        - smoothstep(0.5 - aa, 0.5, radial);

    let triangle_radius = 0.5 - (RING_WIDTH + 2.0 * SPACING) / min_side;
    let triangle_hue = fract(uniform_data.hue / 360.0);
    let hue_angle = radians(uniform_data.hue);
    let white_angle = hue_angle + PI_2 / 3.0;
    let black_angle = hue_angle - PI_2 / 3.0;
    let hue_point = vec2(cos(hue_angle), sin(hue_angle)) * triangle_radius;
    let white_point = vec2(cos(white_angle), sin(white_angle)) * triangle_radius;
    let black_point = vec2(cos(black_angle), sin(black_angle)) * triangle_radius;

    // Calculate WB values
    let area = cross_2d(white_point - hue_point, black_point - hue_point);
    var whiteness = cross_2d(centered - hue_point, black_point - hue_point) / area;
    var blackness = cross_2d(white_point - hue_point, centered - hue_point) / area;

    // Clamp pixels outside triangle, for antialiasing
    whiteness = clamp(whiteness, 0.0, 1.0);
    blackness = clamp(blackness, 0.0, 1.0);
    let wb = whiteness + blackness;
    if wb > 1.0 {
        whiteness /= wb;
        blackness /= wb;
    }

    // Triangle SDF, winding counterclockwise, positive inside
    let triangle_sd = min(
        line_distance(centered, hue_point, white_point),
        min(
            line_distance(centered, white_point, black_point),
            line_distance(centered, black_point, hue_point),
        ),
    );
    let triangle_aa = fwidth(triangle_sd);
    let triangle_alpha = smoothstep(-triangle_aa, 0.0, triangle_sd);

    let alpha = min(ring_alpha + triangle_alpha, 1.0);

#ifdef TRIANGLE_HWB
    let ring_color = hwb_to_linear_rgb(vec3(ring_hue, 0.0, 0.0));
    let triangle_color = hwb_to_linear_rgb(vec3(triangle_hue, whiteness, blackness));
    let color = (ring_color * ring_alpha + triangle_color * triangle_alpha) / max(alpha, 0.001);
    return vec4(color, alpha);
#else
    // Error color
    return vec4(1.0, 0.0, 1.0, alpha);
#endif
}

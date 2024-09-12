#import bevy_ui::ui_vertex_output::UiVertexOutput

@group(1) @binding(0) var<uniform> color: vec4<f32>;
@group(1) @binding(2) var<uniform> blur_radius: f32;

fn erf(p: vec2<f32>) -> vec2<f32> {
    let s = sign(p);
    let a = abs(p);
    var result = 1.0 + (0.278393 + (0.230389 + 0.078108 * (a * a)) * a) * a;
    result = result * result;
    return s - s / (result * result);
}

fn selectCorner(x: f32, y: f32, c: vec4<f32>) -> f32 {
    return mix(mix(c.x, c.y, step(0., x)), mix(c.w, c.z, step(0., x)), step(0., y));
}

// Return the blurred mask along the x dimension.
fn roundedBoxShadowX(x: f32, y: f32, s: f32, corner: f32, halfSize: vec2<f32>) -> f32 {
    let d = min(halfSize.y - corner - abs(y), 0.);
    let c = halfSize.x - corner + sqrt(max(0., corner * corner - d * d));
    let integral = 0.5 + 0.5 * erf((x + vec2(-c, c)) * (sqrt(0.5) / s));
    return integral.y - integral.x;
}

// Return the mask for the shadow of a box from lower to upper.
fn roundedBoxShadow(
    lower: vec2<f32>,
    upper: vec2<f32>,
    point: vec2<f32>,
    sigma: f32,
    corners: vec4<f32>,
) -> f32 {
  // Center everything to make the math easier.
    let center = (lower + upper) * 0.5;
    let halfSize = (upper - lower) * 0.5;
    let p = point - center;

  // The signal is only non-zero in a limited range, so don't waste samples.
    let low = p.y - halfSize.y;
    let high = p.y + halfSize.y;
    let start = clamp(-3. * sigma, low, high);
    let end = clamp(3. * sigma, low, high);

  // Accumulate samples (we can get away with surprisingly few samples).
    let step = (end - start) / 4.0;
    var y = start + step * 0.5;
    var value: f32 = 0.0;

    for (var i = 0; i < 4; i++) {
        let corner = selectCorner(p.x, p.y, corners);
        value += roundedBoxShadowX(p.x, p.y - y, sigma, corner, halfSize) * gaussian(y, sigma) * step;
        y += step;
    }

    return value;
}

@fragment
fn fragment(
    in: UiVertexOutput,
) -> @location(0) vec4<f32> {
    let radius = in.border_widths;
    let point = (in.uv - 0.5) * in.size;
    let g = color.a * roundedBoxShadow(-0.5 * in.size, 0.5 * in.size, point, ratio, max(blur_radius, 0.01), radius);
    return vec4(color.rgb, g);
}




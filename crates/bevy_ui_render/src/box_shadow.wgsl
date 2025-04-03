#import bevy_render::view::View;
#import bevy_render::globals::Globals;

const PI: f32 = 3.14159265358979323846;
const SAMPLES: i32 = #SHADOW_SAMPLES;

@group(0) @binding(0) var<uniform> view: View;

struct BoxShadowVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) point: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) size: vec2<f32>,
    @location(3) @interpolate(flat) radius: vec4<f32>,    
    @location(4) @interpolate(flat) blur: f32,
}

fn gaussian(x: f32, sigma: f32) -> f32 {
    return exp(-(x * x) / (2. * sigma * sigma)) / (sqrt(2. * PI) * sigma);
}

// Approximates the Gauss error function: https://en.wikipedia.org/wiki/Error_function
fn erf(p: vec2<f32>) -> vec2<f32> {
    let s = sign(p);
    let a = abs(p);
    // fourth degree polynomial approximation for erf
    var result = 1.0 + (0.278393 + (0.230389 + 0.078108 * (a * a)) * a) * a;
    result = result * result;
    return s - s / (result * result);
}

// returns the closest corner radius based on the signs of the components of p
fn selectCorner(p: vec2<f32>, c: vec4<f32>) -> f32 {
    return mix(mix(c.x, c.y, step(0., p.x)), mix(c.w, c.z, step(0., p.x)), step(0., p.y));
}

fn horizontalRoundedBoxShadow(x: f32, y: f32, blur: f32, corner: f32, half_size: vec2<f32>) -> f32 {
    let d = min(half_size.y - corner - abs(y), 0.);
    let c = half_size.x - corner + sqrt(max(0., corner * corner - d * d));
    let integral = 0.5 + 0.5 * erf((x + vec2(-c, c)) * (sqrt(0.5) / blur));
    return integral.y - integral.x;
}

fn roundedBoxShadow(
    lower: vec2<f32>,
    upper: vec2<f32>,
    point: vec2<f32>,
    blur: f32,
    corners: vec4<f32>,
) -> f32 {
    let center = (lower + upper) * 0.5;
    let half_size = (upper - lower) * 0.5;
    let p = point - center;
    let low = p.y - half_size.y;
    let high = p.y + half_size.y;
    let start = clamp(-3. * blur, low, high);
    let end = clamp(3. * blur, low, high);
    let step = (end - start) / f32(SAMPLES);
    var y = start + step * 0.5;
    var value: f32 = 0.0;
    for (var i = 0; i < SAMPLES; i++) {
        let corner = selectCorner(p, corners);
        value += horizontalRoundedBoxShadow(p.x, p.y - y, blur, corner, half_size) * gaussian(y, blur) * step;
        y += step;
    }
    return value;
}

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) vertex_color: vec4<f32>,
    @location(3) size: vec2<f32>,
    @location(4) radius: vec4<f32>,
    @location(5) blur: f32,
    @location(6) bounds: vec2<f32>,
) -> BoxShadowVertexOutput {
    var out: BoxShadowVertexOutput;
    out.position = view.clip_from_world * vec4(vertex_position, 1.0);
    out.point = (uv.xy - 0.5) * bounds;
    out.color = vertex_color;
    out.size = size;
    out.radius = radius;
    out.blur = blur;
    return out;
}

@fragment
fn fragment(
    in: BoxShadowVertexOutput,
) -> @location(0) vec4<f32> {
    let g = in.color.a * roundedBoxShadow(-0.5 * in.size, 0.5 * in.size, in.point, max(in.blur, 0.01), in.radius);
    return vec4(in.color.rgb, g);
}




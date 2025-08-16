#import bevy_render::view::View
#import bevy_ui::ui_node::{
    draw_uinode_background,
    draw_uinode_border,
}

const PI: f32 = 3.14159265358979323846;
const TAU: f32 = 2. * PI;
const HUE_GUARD: f32 = 0.0001;

const TEXTURED = 1u;
const RIGHT_VERTEX = 2u;
const BOTTOM_VERTEX = 4u;
// must align with BORDER_* shader_flags from bevy_ui/render/mod.rs
const RADIAL: u32 = 16u;
const FILL_START: u32 = 32u;
const FILL_END: u32 = 64u;
const CONIC: u32 = 128u;
const BORDER_LEFT: u32 = 256u;
const BORDER_TOP: u32 = 512u;
const BORDER_RIGHT: u32 = 1024u;
const BORDER_BOTTOM: u32 = 2048u;
const BORDER_ANY: u32 = BORDER_LEFT + BORDER_TOP + BORDER_RIGHT + BORDER_BOTTOM;

fn enabled(flags: u32, mask: u32) -> bool {
    return (flags & mask) != 0u;
}

@group(0) @binding(0) var<uniform> view: View;

struct GradientVertexOutput {
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) size: vec2<f32>,
    @location(2) @interpolate(flat) flags: u32,
    @location(3) @interpolate(flat) radius: vec4<f32>,
    @location(4) @interpolate(flat) border: vec4<f32>,    

    // Position relative to the center of the rectangle.
    @location(5) point: vec2<f32>,
    @location(6) @interpolate(flat) g_start: vec2<f32>,
    @location(7) @interpolate(flat) dir: vec2<f32>,
    @location(8) @interpolate(flat) start_color: vec4<f32>,
    @location(9) @interpolate(flat) start_len: f32,
    @location(10) @interpolate(flat) end_len: f32,
    @location(11) @interpolate(flat) end_color: vec4<f32>,
    @location(12) @interpolate(flat) hint: f32,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>,
    @location(2) flags: u32,

    // x: top left, y: top right, z: bottom right, w: bottom left.
    @location(3) radius: vec4<f32>,

    // x: left, y: top, z: right, w: bottom.
    @location(4) border: vec4<f32>,
    @location(5) size: vec2<f32>,
    @location(6) point: vec2<f32>,
    @location(7) @interpolate(flat) g_start: vec2<f32>,
    @location(8) @interpolate(flat) dir: vec2<f32>,
    @location(9) @interpolate(flat) start_color: vec4<f32>,
    @location(10) @interpolate(flat) start_len: f32,
    @location(11) @interpolate(flat) end_len: f32,
    @location(12) @interpolate(flat) end_color: vec4<f32>,
    @location(13) @interpolate(flat) hint: f32
) -> GradientVertexOutput {
    var out: GradientVertexOutput;
    out.position = view.clip_from_world * vec4(vertex_position, 1.0);
    out.uv = vertex_uv;
    out.size = size;
    out.flags = flags;
    out.radius = radius;
    out.border = border;
    out.point = point;
    out.dir = dir;
    out.start_color = start_color;
    out.start_len = start_len;
    out.end_len = end_len;
    out.end_color = end_color;
    out.g_start = g_start;
    out.hint = hint;

    return out;
}

@fragment
fn fragment(in: GradientVertexOutput) -> @location(0) vec4<f32> {
    var g_distance: f32;
    if enabled(in.flags, RADIAL) {
        g_distance = radial_distance(in.point, in.g_start, in.dir.x);
    } else if enabled(in.flags, CONIC) {
        g_distance = conic_distance(in.dir.x, in.point, in.g_start);
    } else {
        g_distance = linear_distance(in.point, in.g_start, in.dir);
    }

    let gradient_color = interpolate_gradient(
        g_distance,
        in.start_color,
        in.start_len,
        in.end_color,
        in.end_len,
        in.hint,
        in.flags
    );

    if enabled(in.flags, BORDER_ANY) {
        return draw_uinode_border(gradient_color, in.point, in.size, in.radius, in.border, in.flags);
    } else {
        return draw_uinode_background(gradient_color, in.point, in.size, in.radius, in.border);
    }
}

// https://en.wikipedia.org/wiki/SRGB
fn gamma(value: f32) -> f32 {
    if value <= 0.0 {
        return value;
    }
    if value <= 0.04045 {
        return value / 12.92; // linear falloff in dark values
    } else {
        return pow((value + 0.055) / 1.055, 2.4); // gamma curve in other area
    }
}

// https://en.wikipedia.org/wiki/SRGB
fn inverse_gamma(value: f32) -> f32 {
    if value <= 0.0 {
        return value;
    }

    if value <= 0.0031308 {
        return value * 12.92; // linear falloff in dark values
    } else {
        return 1.055 * pow(value, 1.0 / 2.4) - 0.055; // gamma curve in other area
    }
}

fn srgb_to_linear_rgb(color: vec3<f32>) -> vec3<f32> {
    return vec3(
        gamma(color.x),
        gamma(color.y),
        gamma(color.z)
    );
}

fn linear_rgb_to_srgb(color: vec3<f32>) -> vec3<f32> {
    return vec3(
        inverse_gamma(color.x),
        inverse_gamma(color.y),
        inverse_gamma(color.z)
    );
}

fn oklab_to_linear_rgb(c: vec3<f32>) -> vec3<f32> {
    let l_ = c.x + 0.39633778 * c.y + 0.21580376 * c.z;
    let m_ = c.x - 0.105561346 * c.y - 0.06385417 * c.z;
    let s_ = c.x - 0.08948418 * c.y - 1.2914855 * c.z;
    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;
    return vec3(
        4.0767417 * l - 3.3077116 * m + 0.23096994 * s,
        -1.268438 * l + 2.6097574 * m - 0.34131938 * s,
        -0.0041960863 * l - 0.7034186 * m + 1.7076147 * s,
    );
}

fn hsl_to_linear_rgb(hsl: vec3<f32>) -> vec3<f32> {
    let h = hsl.x;
    let s = hsl.y;
    let l = hsl.z;
    let c = (1.0 - abs(2.0 * l - 1.0)) * s;
    let hp = h * 6.0;
    let x = c * (1.0 - abs(hp % 2.0 - 1.0));
    var r: f32 = 0.0;
    var g: f32 = 0.0;
    var b: f32 = 0.0;
    if 0.0 <= hp && hp < 1.0 {
        r = c; g = x; b = 0.0;
    } else if 1.0 <= hp && hp < 2.0 {
        r = x; g = c; b = 0.0;
    } else if 2.0 <= hp && hp < 3.0 {
        r = 0.0; g = c; b = x;
    } else if 3.0 <= hp && hp < 4.0 {
        r = 0.0; g = x; b = c;
    } else if 4.0 <= hp && hp < 5.0 {
        r = x; g = 0.0; b = c;
    } else if 5.0 <= hp && hp < 6.0 {
        r = c; g = 0.0; b = x;
    }
    let m = l - 0.5 * c;
    return srgb_to_linear_rgb(vec3(r + m, g + m, b + m));
}

fn hsv_to_linear_rgb(hsva: vec3<f32>) -> vec3<f32> {
    let h = hsva.x * 6.0;
    let s = hsva.y;
    let v = hsva.z;
    let c = v * s;
    let x = c * (1.0 - abs(h % 2.0 - 1.0));
    let m = v - c;
    var r: f32 = 0.0;
    var g: f32 = 0.0;
    var b: f32 = 0.0;
    if 0.0 <= h && h < 1.0 {
        r = c; g = x; b = 0.0;
    } else if 1.0 <= h && h < 2.0 {
        r = x; g = c; b = 0.0;
    } else if 2.0 <= h && h < 3.0 {
        r = 0.0; g = c; b = x;
    } else if 3.0 <= h && h < 4.0 {
        r = 0.0; g = x; b = c;
    } else if 4.0 <= h && h < 5.0 {
        r = x; g = 0.0; b = c;
    } else if 5.0 <= h && h < 6.0 {
        r = c; g = 0.0; b = x;
    }
    return srgb_to_linear_rgb(vec3(r + m, g + m, b + m));
}

fn oklch_to_linear_rgb(c: vec3<f32>) -> vec3<f32> {
    let hue = c.z * TAU;
    return oklab_to_linear_rgb(vec3(c.x, c.y * cos(hue), c.y * sin(hue)));
}

fn rem_euclid(a: f32, b: f32) -> f32 {
    return ((a % b) + b) % b;
}


// These functions are used to calculate the distance in gradient space from the start of the gradient to the point.
// The distance in gradient space is then used to interpolate between the start and end colors.

fn linear_distance(
    point: vec2<f32>,
    g_start: vec2<f32>,
    g_dir: vec2<f32>,
) -> f32 {
    return dot(point - g_start, g_dir);
}

fn radial_distance(
    point: vec2<f32>,
    center: vec2<f32>,
    ratio: f32,
) -> f32 {
    let d = point - center;
    return length(vec2(d.x, d.y * ratio));
}

fn conic_distance(
    start: f32,
    point: vec2<f32>,
    center: vec2<f32>,
) -> f32 {
    let d = point - center;
    let angle = atan2(-d.x, d.y) + PI;
    return (((angle - start) % TAU) + TAU) % TAU;
}

fn mix_oklch(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> {
    // If the chroma is close to zero for one of the endpoints, don't interpolate 
    // the hue and instead use the hue of the other endpoint. This allows gradients that smoothly 
    // transition from black or white to a target color without passing through unrelated hues.
    var h = a.z;
    var g = b.z;
    if a.y < HUE_GUARD {
        h = g;
    } else if b.y < HUE_GUARD {
        g = h;
    }

    let hue_diff = g - h;
    if abs(hue_diff) > 0.5 {
        if hue_diff > 0.0 {
            h += (hue_diff - 1.) * t;
        } else {
            h += (hue_diff + 1.) * t;
        }
    } else {
        h += hue_diff * t;
    }
    return vec3(
        mix(a.x, b.x, t),
        mix(a.y, b.y, t),
        fract(h),
    );
}

fn mix_oklch_long(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> {
    var h = a.z;
    var g = b.z;
    if a.y < HUE_GUARD {
        h = g;
    } else if b.y < HUE_GUARD {
        g = h;
    }

    let hue_diff = g - h;
    if abs(hue_diff) < 0.5 {
        if hue_diff >= 0.0 {
            h += (hue_diff - 1.) * t;
        } else {
            h += (hue_diff + 1.) * t;
        }
    } else {
        h += hue_diff * t;
    }
    return vec3(
        mix(a.x, b.x, t),
        mix(a.y, b.y, t),
        fract(h),
    );
}

fn mix_hsl(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> {
    // If the saturation is close to zero for one of the endpoints, don't interpolate 
    // the hue and instead use the hue of the other endpoint. This allows gradients that smoothly 
    // transition from black or white to a target color without passing through unrelated hues.
    var h = a.x;
    var g = b.x;
    if a.y < HUE_GUARD {
        h = g;
    } else if b.y < HUE_GUARD {
        g = h;
    }

    return vec3(
        fract(h + (fract(g - h + 0.5) - 0.5) * t),
        mix(a.y, b.y, t),
        mix(a.z, b.z, t),
    );
}

fn mix_hsl_long(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> {
    var h = a.x;
    var g = b.x;
    if a.y < HUE_GUARD {
        h = g;
    } else if b.y < HUE_GUARD {
        g = h;
    }

    let d = fract(g - h + 0.5) - 0.5;
    return vec3(
        fract(h + (d + select(1., -1., 0. < d)) * t),
        mix(a.y, b.y, t),
        mix(a.z, b.z, t),
    );
}

fn mix_hsv(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> {
    // If the saturation is close to zero for one of the endpoints, don't interpolate 
    // the hue and instead use the hue of the other endpoint. This allows gradients that smoothly 
    // transition from black or white to a target color without passing through unrelated hues.
    var h = a.x;
    var g = b.x;
    if a.y < HUE_GUARD {
        h = g;
    } else if b.y < HUE_GUARD {
        g = h;
    }

    let hue_diff = g - h;
    if abs(hue_diff) > 0.5 {
        if hue_diff > 0.0 {
            h += (hue_diff - 1.0) * t;
        } else {
            h += (hue_diff + 1.0) * t;
        }
    } else {
        h += hue_diff * t;
    }
    return vec3(
        fract(h),
        mix(a.y, b.y, t),
        mix(a.z, b.z, t),
    );
}

fn mix_hsv_long(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> {
    var h = a.x;
    var g = b.x;
    if a.y < HUE_GUARD {
        h = g;
    } else if b.y < HUE_GUARD {
        g = h;
    }

    let hue_diff = g - h;
    if abs(hue_diff) < 0.5 {
        if hue_diff >= 0.0 {
            h += (hue_diff - 1.0) * t;
        } else {
            h += (hue_diff + 1.0) * t;
        }
    } else {
        h += hue_diff * t;
    }
    return vec3(
        fract(h),
        mix(a.y, b.y, t),
        mix(a.z, b.z, t),
    );
}

fn interpolate_gradient(
    distance: f32,
    start_color: vec4<f32>,
    start_distance: f32,
    end_color: vec4<f32>,
    end_distance: f32,
    hint: f32,
    flags: u32,
) -> vec4<f32> {
    if start_distance == end_distance {
        if distance <= start_distance && enabled(flags, FILL_START) {
            return convert_to_linear_rgba(start_color);
        }
        if start_distance <= distance && enabled(flags, FILL_END) {
            return convert_to_linear_rgba(end_color);
        }
        return vec4(0.);
    }

    var t = (distance - start_distance) / (end_distance - start_distance);

    if t < 0.0 {
        if enabled(flags, FILL_START) {
            return convert_to_linear_rgba(start_color);
        }
        return vec4(0.0);
    }

    if 1. < t {
        if enabled(flags, FILL_END) {
            return convert_to_linear_rgba(end_color);
        }
        return vec4(0.0);
    }

    if t < hint {
        t = 0.5 * t / hint;
    } else {
        t = 0.5 * (1 + (t - hint) / (1.0 - hint));
    }

    return convert_to_linear_rgba(vec4(mix_colors(start_color.xyz, end_color.xyz, t), mix(start_color.a, end_color.a, t)));
}

// Mix the colors, choosing the appropriate interpolation method for the given color space
fn mix_colors(
    start_color: vec3<f32>,
    end_color: vec3<f32>,
    t: f32,
) -> vec3<f32> {
#ifdef IN_OKLCH
    return mix_oklch(start_color, end_color, t);
#else ifdef IN_OKLCH_LONG
    return mix_oklch_long(start_color, end_color, t);
#else ifdef IN_HSV
    return mix_hsv(start_color, end_color, t);
#else ifdef IN_HSV_LONG
    return mix_hsv_long(start_color, end_color, t);
#else ifdef IN_HSL
    return mix_hsl(start_color, end_color, t);
#else ifdef IN_HSL_LONG
    return mix_hsl_long(start_color, end_color, t);
#else
    // Just lerp in linear RGBA, OkLab and SRGBA spaces
    return mix(start_color, end_color, t);
#endif
}

// Convert a color from the interpolation color space to linear rgba
fn convert_to_linear_rgba(
    color: vec4<f32>,
) -> vec4<f32> {
#ifdef IN_OKLCH
    let rgb = oklch_to_linear_rgb(color.xyz);
#else ifdef IN_OKLCH_LONG
    let rgb = oklch_to_linear_rgb(color.xyz);
#else ifdef IN_HSV
    let rgb = hsv_to_linear_rgb(color.xyz);
#else ifdef IN_HSV_LONG
    let rgb = hsv_to_linear_rgb(color.xyz);
#else ifdef IN_HSL
    let rgb = hsl_to_linear_rgb(color.xyz);
#else ifdef IN_HSL_LONG
    let rgb = hsl_to_linear_rgb(color.xyz);
#else ifdef IN_OKLAB
    let rgb = oklab_to_linear_rgb(color.xyz);
#else ifdef IN_SRGB
    let rgb = pow(color.xyz, vec3(2.2));
#else
    // Color is already in linear rgba space
    let rgb = color.rgb;
#endif
    return vec4(rgb, color.a);
}

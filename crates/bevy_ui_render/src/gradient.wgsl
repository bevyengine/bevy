#import bevy_render::view::View
#import bevy_ui::ui_node::{
    draw_uinode_background,
    draw_uinode_border,
}

const PI: f32 = 3.14159265358979323846;
const TAU: f32 = 2. * PI;

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

// This function converts two linear rgba colors to srgba space, mixes them, and then converts the result back to linear rgb space.
fn mix_linear_rgba_in_srgba_space(a: vec4<f32>, b: vec4<f32>, t: f32) -> vec4<f32> {
    let a_srgb = pow(a.rgb, vec3(1. / 2.2));
    let b_srgb = pow(b.rgb, vec3(1. / 2.2));
    let mixed_srgb = mix(a_srgb, b_srgb, t);
    return vec4(pow(mixed_srgb, vec3(2.2)), mix(a.a, b.a, t));
}

fn linear_rgba_to_oklaba(c: vec4<f32>) -> vec4<f32> {
    let l = pow(0.41222146 * c.x + 0.53633255 * c.y + 0.051445995 * c.z, 1. / 3.);
    let m = pow(0.2119035 * c.x + 0.6806995 * c.y + 0.10739696 * c.z, 1. / 3.);
    let s = pow(0.08830246 * c.x + 0.28171885 * c.y + 0.6299787 * c.z, 1. / 3.);
    return vec4(
        0.21045426 * l + 0.7936178 * m - 0.004072047 * s,
        1.9779985 * l - 2.4285922 * m + 0.4505937 * s,
        0.025904037 * l + 0.78277177 * m - 0.80867577 * s,
        c.a
    );
}

fn oklaba_to_linear_rgba(c: vec4<f32>) -> vec4<f32> {
    let l_ = c.x + 0.39633778 * c.y + 0.21580376 * c.z;
    let m_ = c.x - 0.105561346 * c.y - 0.06385417 * c.z;
    let s_ = c.x - 0.08948418 * c.y - 1.2914855 * c.z;
    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;
    return vec4(
        4.0767417 * l - 3.3077116 * m + 0.23096994 * s,
        -1.268438 * l + 2.6097574 * m - 0.34131938 * s,
        -0.0041960863 * l - 0.7034186 * m + 1.7076147 * s,
        c.a
    );
}

fn mix_linear_rgba_in_oklaba_space(a: vec4<f32>, b: vec4<f32>, t: f32) -> vec4<f32> {
    return oklaba_to_linear_rgba(mix(linear_rgba_to_oklaba(a), linear_rgba_to_oklaba(b), t));
}

fn linear_rgba_to_hsla(c: vec4<f32>) -> vec4<f32> {
    let max = max(max(c.r, c.g), c.b);
    let min = min(min(c.r, c.g), c.b);
    let l = (max + min) * 0.5;
    if max == min {
        return vec4(0., 0., l, c.a);
    } else {
        let delta = max - min;
        let s = delta / (1. - abs(2. * l - 1.));
        var h = 0.;
        if max == c.r {
            h = ((c.g - c.b) / delta) % 6.;
        } else if max == c.g {
            h = ((c.b - c.r) / delta) + 2.;
        } else {
            h = ((c.r - c.g) / delta) + 4.;
        }
        h = h / 6.;
        return vec4<f32>(h, s, l, c.a);
    }
}


fn hsla_to_linear_rgba(hsl: vec4<f32>) -> vec4<f32> {
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
    return vec4<f32>(r + m, g + m, b + m, hsl.a);
}

fn linear_rgba_to_hsva(c: vec4<f32>) -> vec4<f32> {
    let maxc = max(max(c.r, c.g), c.b);
    let minc = min(min(c.r, c.g), c.b);
    let delta = maxc - minc;
    var h: f32 = 0.0;
    var s: f32 = 0.0;
    let v: f32 = maxc;
    if delta != 0.0 {
        s = delta / maxc;
        if maxc == c.r {
            h = ((c.g - c.b) / delta) % 6.0;
        } else if maxc == c.g {
            h = ((c.b - c.r) / delta) + 2.0;
        } else {
            h = ((c.r - c.g) / delta) + 4.0;
        }
        h = h / 6.0;
        if h < 0.0 {
            h = h + 1.0;
        }
    }
    return vec4<f32>(h, s, v, c.a);
}

fn hsva_to_linear_rgba(hsva: vec4<f32>) -> vec4<f32> {
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
    return vec4<f32>(r + m, g + m, b + m, hsva.a);
}

/// hue is left in radians and not converted to degrees
fn linear_rgba_to_oklcha(c: vec4<f32>) -> vec4<f32> {
    let o = linear_rgba_to_oklaba(c);
    let chroma = sqrt(o.y * o.y + o.z * o.z);
    let hue = atan2(o.z, o.y);
    return vec4(o.x, chroma, rem_euclid(hue, TAU), o.a);
}

fn oklcha_to_linear_rgba(c: vec4<f32>) -> vec4<f32> {
    let a = c.y * cos(c.z);
    let b = c.y * sin(c.z);
    return oklaba_to_linear_rgba(vec4(c.x, a, b, c.a));
}

fn rem_euclid(a: f32, b: f32) -> f32 {
    return ((a % b) + b) % b;
}

fn lerp_hue(a: f32, b: f32, t: f32) -> f32 {
    let diff = rem_euclid(b - a + PI, TAU) - PI;
    return rem_euclid(a + diff * t, TAU);
}

fn lerp_hue_long(a: f32, b: f32, t: f32) -> f32 {
    let diff = rem_euclid(b - a + PI, TAU) - PI;
    return rem_euclid(a + (diff + select(TAU, -TAU, 0. < diff)) * t, TAU);
}

fn mix_oklcha(a: vec4<f32>, b: vec4<f32>, t: f32) -> vec4<f32> {
    let ah = select(a.z, b.z, a.y == 0.);
    let bh = select(b.z, a.z, b.y == 0.);
    return vec4(
        mix(a.xy, b.xy, t),
        lerp_hue(ah, bh, t),
        mix(a.a, b.a, t)
    );
}

fn mix_oklcha_long(a: vec4<f32>, b: vec4<f32>, t: f32) -> vec4<f32> {
    let ah = select(a.z, b.z, a.y == 0.);
    let bh = select(b.z, a.z, b.y == 0.);
    return vec4(
        mix(a.xy, b.xy, t),
        lerp_hue_long(ah, bh, t),
        mix(a.w, b.w, t)
    );
}

fn mix_linear_rgba_in_oklcha_space(a: vec4<f32>, b: vec4<f32>, t: f32) -> vec4<f32> {
    return oklcha_to_linear_rgba(mix_oklcha(linear_rgba_to_oklcha(a), linear_rgba_to_oklcha(b), t));
}

fn mix_linear_rgba_in_oklcha_space_long(a: vec4<f32>, b: vec4<f32>, t: f32) -> vec4<f32> {
    return oklcha_to_linear_rgba(mix_oklcha_long(linear_rgba_to_oklcha(a), linear_rgba_to_oklcha(b), t));
}

fn mix_linear_rgba_in_hsva_space(a: vec4<f32>, b: vec4<f32>, t: f32) -> vec4<f32> {
    let ha = linear_rgba_to_hsva(a);
    let hb = linear_rgba_to_hsva(b);
    var h: f32;
    if ha.y == 0. {
        h = hb.x;
    } else if hb.y == 0. {
        h = ha.x;
    } else {
        h = lerp_hue(ha.x * TAU, hb.x * TAU, t) / TAU;
    }
    let s = mix(ha.y, hb.y, t);
    let v = mix(ha.z, hb.z, t);
    let a_alpha = mix(ha.a, hb.a, t);
    return hsva_to_linear_rgba(vec4<f32>(h, s, v, a_alpha));
}

fn mix_linear_rgba_in_hsva_space_long(a: vec4<f32>, b: vec4<f32>, t: f32) -> vec4<f32> {
    let ha = linear_rgba_to_hsva(a);
    let hb = linear_rgba_to_hsva(b);
    let h = lerp_hue_long(ha.x * TAU, hb.x * TAU, t) / TAU;
    let s = mix(ha.y, hb.y, t);
    let v = mix(ha.z, hb.z, t);
    let a_alpha = mix(ha.a, hb.a, t);
    return hsva_to_linear_rgba(vec4<f32>(h, s, v, a_alpha));
}

fn mix_linear_rgba_in_hsla_space(a: vec4<f32>, b: vec4<f32>, t: f32) -> vec4<f32> {
    let ha = linear_rgba_to_hsla(a);
    let hb = linear_rgba_to_hsla(b);
    let h = lerp_hue(ha.x * TAU, hb.x * TAU, t) / TAU;
    let s = mix(ha.y, hb.y, t);
    let l = mix(ha.z, hb.z, t);
    let a_alpha = mix(ha.a, hb.a, t);
    return hsla_to_linear_rgba(vec4<f32>(h, s, l, a_alpha));
}

fn mix_linear_rgba_in_hsla_space_long(a: vec4<f32>, b: vec4<f32>, t: f32) -> vec4<f32> {
    let ha = linear_rgba_to_hsla(a);
    let hb = linear_rgba_to_hsla(b);
    let h = lerp_hue_long(ha.x * TAU, hb.x * TAU, t) / TAU;
    let s = mix(ha.y, hb.y, t);
    let l = mix(ha.z, hb.z, t);
    let a_alpha = mix(ha.a, hb.a, t);
    return hsla_to_linear_rgba(vec4<f32>(h, s, l, a_alpha));
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
            return start_color;
        }
        if start_distance <= distance && enabled(flags, FILL_END) {
            return end_color;
        }
        return vec4(0.);
    }

    var t = (distance - start_distance) / (end_distance - start_distance);

    if t < 0.0 {
        if enabled(flags, FILL_START) {
            return start_color;
        }
        return vec4(0.0);
    }

    if 1. < t {
        if enabled(flags, FILL_END) {
            return end_color;
        }
        return vec4(0.0);
    }

    if t < hint {
        t = 0.5 * t / hint;
    } else {
        t = 0.5 * (1 + (t - hint) / (1.0 - hint));
    }
    
#ifdef IN_SRGB
    return mix_linear_rgba_in_srgba_space(start_color, end_color, t);
#else ifdef IN_OKLAB
    return mix_linear_rgba_in_oklaba_space(start_color, end_color, t);
#else ifdef IN_OKLCH
    return mix_linear_rgba_in_oklcha_space(start_color, end_color, t);
#else ifdef IN_OKLCH_LONG
    return mix_linear_rgba_in_oklcha_space_long(start_color, end_color, t);
#else ifdef IN_HSV
    return mix_linear_rgba_in_hsva_space(start_color, end_color, t);
#else ifdef IN_HSV_LONG
    return mix_linear_rgba_in_hsva_space_long(start_color, end_color, t);
#else ifdef IN_HSL
    return mix_linear_rgba_in_hsla_space(start_color, end_color, t);
#else ifdef IN_HSL_LONG
    return mix_linear_rgba_in_hsla_space_long(start_color, end_color, t);
#else
    return mix(start_color, end_color, t);
#endif
}

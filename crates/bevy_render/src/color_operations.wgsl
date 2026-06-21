#define_import_path bevy_render::color_operations

#import bevy_render::maths::{PI_2,PI,FRAC_PI_3}

const HUE_GUARD: f32 = 0.0001;

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

fn linear_to_srgb(color: vec3<f32>) -> vec3<f32> {
    return linear_rgb_to_srgb(color);
}

fn srgb_to_linear(color: vec3<f32>) -> vec3<f32> {
    return srgb_to_linear_rgb(color);
}

// https://bottosson.github.io/posts/oklab/
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

fn okhsl_to_oklab(okhsl: vec3<f32>) -> vec3<f32> {
    let h = okhsl.x;
    let s = okhsl.y;
    let l = okhsl.z;

    if (l >= 1.0) {
        return vec3<f32>(1.0, 0.0, 0.0);
    }
    if (l <= 0.0) {
        return vec3<f32>(0.0, 0.0, 0.0);
    }

    let a_ = cos(2.0 * PI * h);
    let b_ = sin(2.0 * PI * h);
    let L = okhsl_toe_inv(l);

    let cs = okhsl_get_Cs(L, a_, b_);
    let C_0 = cs.x;
    let C_mid = cs.y;
    let C_max = cs.z;

    let mid = 0.8;
    let mid_inv = 1.25;

    var C: f32 = 0.0;
    var t: f32 = 0.0;
    var k_0: f32 = 0.0;
    var k_1: f32 = 0.0;
    var k_2: f32 = 0.0;

    if (s < mid) {
        t = mid_inv * s;
        k_1 = mid * C_0;
        k_2 = 1.0 - k_1 / C_mid;
        C = t * k_1 / (1.0 - k_2 * t);
    } else {
        t = (s - mid) / (1.0 - mid);
        k_0 = C_mid;
        k_1 = (1.0 - mid) * C_mid * C_mid * mid_inv * mid_inv / C_0;
        k_2 = 1.0 - k_1 / (C_max - C_mid);
        C = k_0 + t * k_1 / (1.0 - k_2 * t);
    }

    return vec3<f32>(L, C * a_, C * b_);
}

fn okhsl_to_linear_rgb(okhsl: vec3<f32>) -> vec3<f32> {
    let oklab = okhsl_to_oklab(okhsl);
    return oklab_to_linear_rgb(oklab);
}

// https://bottosson.github.io/posts/oklab/ - inverse of oklab_to_linear_rgb
fn linear_rgb_to_oklab(c: vec3<f32>) -> vec3<f32> {
    let l_ = pow(c.x * 0.4122214708 + c.y * 0.5363325363 + c.z * 0.0514459929, 1.0 / 3.0);
    let m_ = pow(c.x * 0.2119034982 + c.y * 0.6806995451 + c.z * 0.1073969566, 1.0 / 3.0);
    let s_ = pow(c.x * 0.0883024619 + c.y * 0.2817188376 + c.z * 0.6299787005, 1.0 / 3.0);
    return vec3(
        0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
        1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
        0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_,
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
    let hue = c.z * PI_2;
    return oklab_to_linear_rgb(vec3(c.x, c.y * cos(hue), c.y * sin(hue)));
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

fn mix_okhsl(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> {
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

fn mix_okhsl_long(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> {
    var h = a.x;
    var g = b.x;

    if a.y < HUE_GUARD {
        h = g;
    } else if b.y < HUE_GUARD {
        g = h;
    }

    let d = fract(g - h + 0.5) - 0.5;
    return vec3(
        fract(h + (d + select(1.0, -1.0, 0.0 < d)) * t),
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

// Converts HSV to RGB.
//
// Input: H ∈ [0, 2π), S ∈ [0, 1], V ∈ [0, 1].
// Output: R ∈ [0, 1], G ∈ [0, 1], B ∈ [0, 1].
//
// <https://en.wikipedia.org/wiki/HSL_and_HSV#HSV_to_RGB_alternative>
fn hsv_to_rgb(hsv: vec3<f32>) -> vec3<f32> {
    let n = vec3(5.0, 3.0, 1.0);
    let k = (n + hsv.x / FRAC_PI_3) % 6.0;
    return hsv.z - hsv.z * hsv.y * max(vec3(0.0), min(k, min(4.0 - k, vec3(1.0))));
}

// Converts RGB to HSV.
//
// Input: R ∈ [0, 1], G ∈ [0, 1], B ∈ [0, 1].
// Output: H ∈ [0, 2π), S ∈ [0, 1], V ∈ [0, 1].
//
// <https://en.wikipedia.org/wiki/HSL_and_HSV#From_RGB>
fn rgb_to_hsv(rgb: vec3<f32>) -> vec3<f32> {
    let x_max = max(rgb.r, max(rgb.g, rgb.b));  // i.e. V
    let x_min = min(rgb.r, min(rgb.g, rgb.b));
    let c = x_max - x_min;  // chroma

    var swizzle = vec3<f32>(0.0);
    if (x_max == rgb.r) {
        swizzle = vec3(rgb.gb, 0.0);
    } else if (x_max == rgb.g) {
        swizzle = vec3(rgb.br, 2.0);
    } else {
        swizzle = vec3(rgb.rg, 4.0);
    }

    let h = FRAC_PI_3 * fract(((swizzle.x - swizzle.y) / c + swizzle.z) / 6.0) * 6.0;

    // Avoid division by zero.
    var s = 0.0;
    if (x_max > 0.0) {
        s = c / x_max;
    }

    return vec3(h, s, x_max);
}

// --- OKHSL conversion helpers ---
// Ported from crates/bevy_color/src/okcolor_convert.rs

fn okhsl_toe_inv(x: f32) -> f32 {
    let k_1 = 0.206;
    let k_2 = 0.03;
    let k_3 = (1.0 + k_1) / (1.0 + k_2);
    return (x * x + k_1 * x) / (k_3 * (x + k_2));
}

fn okhsl_to_ST(cusp: vec2<f32>) -> vec2<f32> {
    // cusp.x = L, cusp.y = C
    return vec2<f32>(cusp.y / cusp.x, cusp.y / (1.0 - cusp.x));
}

// Polynomial approximation for mid-gamut ST values.
// Derived by optimization to ensure S_mid < S_max and T_mid < T_max.
fn okhsl_get_ST_mid(a_: f32, b_: f32) -> vec2<f32> {
    let S = 0.11516993
        + 1.0 / (7.4477897
            + 4.1590123 * b_
            + a_ * (-2.1955736
                + 1.751984 * b_
                + a_ * (-2.1370494 - 10.02301 * b_
                    + a_ * (-4.2489457 + 5.387708 * b_ + 4.69891 * a_))));

    let T = 0.11239642
        + 1.0 / (1.6132032 - 0.6812438 * b_
            + a_ * (0.40370612
                + 0.9014812 * b_
                + a_ * (-0.27087943
                    + 0.6122399 * b_
                    + a_ * (0.00299215 - 0.45399568 * b_ - 0.14661872 * a_))));

    return vec2<f32>(S, T);
}

// Approximate max saturation using a polynomial, then refine with 1 step of Halley's method.
// a and b must be normalized (a^2 + b^2 == 1).
fn okhsl_compute_max_saturation(a: f32, b: f32) -> f32 {
    var k0: f32 = 0.0; var k1: f32 = 0.0; var k2: f32 = 0.0;
    var k3: f32 = 0.0; var k4: f32 = 0.0;
    var wl: f32 = 0.0; var wm: f32 = 0.0; var ws: f32 = 0.0;

    if (-1.8817033 * a - 0.8093649 * b > 1.0) {
        // Red component limit
        k0 = 1.1908628; k1 = 1.7657673; k2 = 0.5966264;
        k3 = 0.755152; k4 = 0.5677124;
        wl = 4.0767417; wm = -3.3077116; ws = 0.23096994;
    } else if (1.8144411 * a - 1.1944528 * b > 1.0) {
        // Green component limit
        k0 = 0.73956515; k1 = -0.45954404; k2 = 0.08285427;
        k3 = 0.1254107; k4 = 0.14503204;
        wl = -1.268438; wm = 2.6097574; ws = -0.34131938;
    } else {
        // Blue component limit
        k0 = 1.3573365; k1 = -0.00915799; k2 = -1.1513021;
        k3 = -0.50559606; k4 = 0.00692167;
        wl = -0.0041960863; wm = -0.7034186; ws = 1.7076147;
    }

    var S = k0 + k1 * a + k2 * b + k3 * a * a + k4 * a * b;

    let k_l = 0.39633778 * a + 0.21580376 * b;
    let k_m = -0.105561346 * a - 0.06385417 * b;
    let k_s = -0.08948418 * a - 1.2914855 * b;
    
    let l_ = 1.0 + S * k_l;
    let m_ = 1.0 + S * k_m;
    let s_ = 1.0 + S * k_s;

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    let l_dS = 3.0 * k_l * l_ * l_;
    let m_dS = 3.0 * k_m * m_ * m_;
    let s_dS = 3.0 * k_s * s_ * s_;

    let l_dS2 = 6.0 * k_l * k_l * l_;
    let m_dS2 = 6.0 * k_m * k_m * m_;
    let s_dS2 = 6.0 * k_s * k_s * s_;

    let f = wl * l + wm * m + ws * s;
    let f1 = wl * l_dS + wm * m_dS + ws * s_dS;
    let f2 = wl * l_dS2 + wm * m_dS2 + ws * s_dS2;

    S = S - f * f1 / (f1 * f1 - 0.5 * f * f2);
    return S;
}

fn okhsl_find_cusp(a: f32, b: f32) -> vec2<f32> {
    let S_cusp = okhsl_compute_max_saturation(a, b);
    let rgb_at_max = oklab_to_linear_rgb(vec3<f32>(1.0, S_cusp * a, S_cusp * b));
    let L_cusp = pow(1.0 / max(rgb_at_max.r, max(rgb_at_max.g, rgb_at_max.b)), 1.0 / 3.0);
    return vec2<f32>(L_cusp, L_cusp * S_cusp);
}

// Find intersection of the gamut boundary with the line from (L0, 0) to (L1, C1).
// Uses 1 step of Halley's method.
fn okhsl_find_gamut_intersection(a: f32, b: f32, L1: f32, C1: f32, L0: f32, cusp: vec2<f32>) -> f32 {
    let cusp_L = cusp.x;
    let cusp_C = cusp.y;
    var t: f32;

    if (((L1 - L0) * cusp_C - (cusp_L - L0) * C1) <= 0.0) {
        // Lower half of the gamut
        t = cusp_C * L0 / (C1 * cusp_L + cusp_C * (L0 - L1));
    } else {
        // Upper half: first intersect with triangle, then refine
        t = cusp_C * (L0 - 1.0) / (C1 * (cusp_L - 1.0) + cusp_C * (L0 - L1));

        let dL = L1 - L0;
        let dC = C1;

        let k_l = 0.39633778 * a + 0.21580376 * b;
        let k_m = -0.105561346 * a - 0.06385417 * b;
        let k_s = -0.08948418 * a - 1.2914855 * b;

        let l_dt = dL + dC * k_l;
        let m_dt = dL + dC * k_m;
        let s_dt = dL + dC * k_s;

        for (var i = 0; i < 1; i = i + 1) {
            let L = L0 * (1.0 - t) + t * L1;
            let C = t * C1;

            let l_ = L + C * k_l;
            let m_ = L + C * k_m;
            let s_ = L + C * k_s;

            let l = l_ * l_ * l_;
            let m = m_ * m_ * m_;
            let s = s_ * s_ * s_;

            let ldt = 3.0 * l_dt * l_ * l_;
            let mdt = 3.0 * m_dt * m_ * m_;
            let sdt = 3.0 * s_dt * s_ * s_;

            let ldt2 = 6.0 * l_dt * l_dt * l_;
            let mdt2 = 6.0 * m_dt * m_dt * m_;
            let sdt2 = 6.0 * s_dt * s_dt * s_;

            let r = 4.0767417 * l - 3.3077116 * m + 0.23096994 * s - 1.0;
            let r1 = 4.0767417 * ldt - 3.3077116 * mdt + 0.23096994 * sdt;
            let r2 = 4.0767417 * ldt2 - 3.3077116 * mdt2 + 0.23096994 * sdt2;

            let u_r = r1 / (r1 * r1 - 0.5 * r * r2);
            let t_r = select(3.40282347e+38, -r * u_r, u_r >= 0.0);

            let g = -1.268438 * l + 2.6097574 * m - 0.34131938 * s - 1.0;
            let g1 = -1.268438 * ldt + 2.6097574 * mdt - 0.34131938 * sdt;
            let g2 = -1.268438 * ldt2 + 2.6097574 * mdt2 - 0.34131938 * sdt2;

            let u_g = g1 / (g1 * g1 - 0.5 * g * g2);
            let t_g = select(3.40282347e+38, -g * u_g, u_g >= 0.0);

            let b_val = -0.0041960863 * l - 0.7034186 * m + 1.7076147 * s - 1.0;
            let b1 = -0.0041960863 * ldt - 0.7034186 * mdt + 1.7076147 * sdt;
            let b2 = -0.0041960863 * ldt2 - 0.7034186 * mdt2 + 1.7076147 * sdt2;

            let u_b = b1 / (b1 * b1 - 0.5 * b_val * b2);
            let t_b = select(3.40282347e+38, -b_val * u_b, u_b >= 0.0);

            t = t + min(t_r, min(t_g, t_b));
        }
    }
    return t;
}

// Returns C_0, C_mid, C_max for a given Oklab L and hue direction (a_, b_).
fn okhsl_get_Cs(L: f32, a_: f32, b_: f32) -> vec3<f32> {
    let cusp = okhsl_find_cusp(a_, b_);
    let C_max = okhsl_find_gamut_intersection(a_, b_, L, 1.0, L, cusp);
    let ST_max = okhsl_to_ST(cusp);

    let k = C_max / min(L * ST_max.x, (1.0 - L) * ST_max.y);

    let ST_mid = okhsl_get_ST_mid(a_, b_);
    let C_a = L * ST_mid.x;
    let C_b = (1.0 - L) * ST_mid.y;
    let C_mid = 0.9 * k * sqrt(sqrt(1.0 / (1.0 / (C_a * C_a * C_a * C_a) + 1.0 / (C_b * C_b * C_b * C_b))));

    let C_0_a = L * 0.4;
    let C_0_b = (1.0 - L) * 0.8;
    let C_0 = sqrt(1.0 / (1.0 / (C_0_a * C_0_a) + 1.0 / (C_0_b * C_0_b)));

    return vec3<f32>(C_0, C_mid, C_max);
}

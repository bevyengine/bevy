#define_import_path bevy_ui_render::color_space
#import bevy_render::maths::PI

const TAU: f32 = 2. * PI;
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

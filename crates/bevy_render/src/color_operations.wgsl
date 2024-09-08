#define_import_path bevy_render::color_operations

#import bevy_render::maths::FRAC_PI_3

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

    let h = FRAC_PI_3 * (((swizzle.x - swizzle.y) / c + swizzle.z) % 6.0);

    // Avoid division by zero.
    var s = 0.0;
    if (x_max > 0.0) {
        s = c / x_max;
    }

    return vec3(h, s, x_max);
}


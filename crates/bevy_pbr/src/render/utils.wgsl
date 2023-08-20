#define_import_path bevy_pbr::utils

const PI: f32 = 3.141592653589793;
const HALF_PI: f32 = 1.57079632679;
const E: f32 = 2.718281828459045;

fn hsv2rgb(hue: f32, saturation: f32, value: f32) -> vec3<f32> {
    let rgb = clamp(
        abs(
            ((hue * 6.0 + vec3<f32>(0.0, 4.0, 2.0)) % 6.0) - 3.0
        ) - 1.0,
        vec3<f32>(0.0),
        vec3<f32>(1.0)
    );

    return value * mix(vec3<f32>(1.0), rgb, vec3<f32>(saturation));
}

fn random1D(s: f32) -> f32 {
    return fract(sin(s * 12.9898) * 43758.5453123);
}

// returns the (0-1, 0-1) position within the given viewport for the current buffer coords .
// buffer coords can be obtained from `@builtin(position).xy`.
// the view uniform struct contains the current camera viewport in `view.viewport`.
// topleft = 0,0
fn coords_to_viewport_uv(position: vec2<f32>, viewport: vec4<f32>) -> vec2<f32> {
    return (position - viewport.xy) / viewport.zw;
}

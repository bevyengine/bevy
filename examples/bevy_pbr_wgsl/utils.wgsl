#define_import_path bevy_pbr::utils

const PI: f32 = 3.141592653589793;

fn saturate(value: f32) -> f32 {
    return clamp(value, 0.0, 1.0);
}

fn hsv2rgb(hue: f32, saturation: f32, value: f32) -> vec3<f32> {
    let rgb = clamp(
        abs(
            ((hue * 6.0 + vec3<f32>(0.0, 4.0, 2.0)) % 6.0) - 3.0
        ) - 1.0,
        vec3<f32>(0.0),
        vec3<f32>(1.0)
    );

	return value * mix( vec3<f32>(1.0), rgb, vec3<f32>(saturation));
}

fn random1D(s: f32) -> f32 {
    return fract(sin(s * 12.9898) * 43758.5453123);
}

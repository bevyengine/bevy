#define_import_path bevy_pbr::fog

// Fog formulas adapted from:
// https://learn.microsoft.com/en-us/windows/win32/direct3d9/fog-formulas
// https://catlikecoding.com/unity/tutorials/rendering/part-14/

fn linear_fog(
    distance: f32,
) -> vec4<f32> {
    var result = fog.color;
    result.a *= 1.0 - clamp((fog.end - distance) / (fog.end - fog.density_or_start), 0.0, 1.0);
    return result;
}

fn exponential_fog(
    distance: f32,
) -> vec4<f32> {
    var result = fog.color;
    result.a *= 1.0 - 1.0 / exp(distance * fog.density_or_start);
    return result;
}

fn exponential_squared_fog(
    distance: f32,
) -> vec4<f32> {
    var result = fog.color;
    result.a *= 1.0 - 1.0 / exp(pow(distance * fog.density_or_start, 2.0));
    return result;
}

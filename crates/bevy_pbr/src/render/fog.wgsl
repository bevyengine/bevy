#define_import_path bevy_pbr::fog

// Fog formulas adapted from:
// https://learn.microsoft.com/en-us/windows/win32/direct3d9/fog-formulas
// https://catlikecoding.com/unity/tutorials/rendering/part-14/

fn linear_fog(
    input_color: vec4<f32>,
    distance: f32,
    scattering: vec3<f32>,
) -> vec4<f32> {
    var result = fog.base_color;
    if (fog.directional_light_color.a > 0.0) {
        result = vec4<f32>(fog.base_color.rgb + scattering * fog.directional_light_color.rgb * fog.directional_light_color.a, result.a);
    }
    let start = fog.be.x;
    let end = fog.be.y;
    result.a *= 1.0 - clamp((end - distance) / (end - start), 0.0, 1.0);
    return vec4<f32>(mix(input_color.rgb, result.rgb, result.a), input_color.a);
}

fn exponential_fog(
    input_color: vec4<f32>,
    distance: f32,
    scattering: vec3<f32>,
) -> vec4<f32> {
    var result = fog.base_color;
    if (fog.directional_light_color.a > 0.0) {
        result = vec4<f32>(fog.base_color.rgb + scattering * fog.directional_light_color.rgb * fog.directional_light_color.a, result.a);
    }
    let density = fog.be.x;
    result.a *= 1.0 - 1.0 / exp(distance * density);
    return vec4<f32>(mix(input_color.rgb, result.rgb, result.a), input_color.a);
}

fn exponential_squared_fog(
    input_color: vec4<f32>,
    distance: f32,
    scattering: vec3<f32>,
) -> vec4<f32> {
    var result = fog.base_color;
    if (fog.directional_light_color.a > 0.0) {
        result = vec4<f32>(fog.base_color.rgb + scattering * fog.directional_light_color.rgb * fog.directional_light_color.a, result.a);
    }
    let density = fog.be.x;
    result.a *= 1.0 - 1.0 / exp(pow(distance * density, 2.0));
    return vec4<f32>(mix(input_color.rgb, result.rgb, result.a), input_color.a);
}

// Fog formula adapted from:
// https://iquilezles.org/articles/fog/

fn atmospheric_fog(
    input_color: vec4<f32>,
    distance: f32,
    scattering: vec3<f32>,
) -> vec4<f32> {
    var result = fog.base_color;
    if (fog.directional_light_color.a > 0.0) {
        result = vec4<f32>(fog.base_color.rgb + scattering * fog.directional_light_color.rgb * fog.directional_light_color.a, result.a);
    }
    let extinction_color = 1.0 - 1.0 / exp(distance * fog.be);
    let indirectional_light_color = 1.0 - 1.0 / exp(distance * fog.bi);

    return vec4<f32>(input_color.rgb * (1.0 - extinction_color * result.a) + result.rgb * indirectional_light_color * result.a, input_color.a);
}

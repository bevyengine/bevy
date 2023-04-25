#define_import_path bevy_pbr::fog

// Fog formulas adapted from:
// https://learn.microsoft.com/en-us/windows/win32/direct3d9/fog-formulas
// https://catlikecoding.com/unity/tutorials/rendering/part-14/
// https://iquilezles.org/articles/fog/ (Atmospheric Fog and Scattering)

fn scattering_adjusted_fog_color(
    fog: Fog,
    scattering: vec3<f32>,
) -> vec4<f32> {
    if (fog.directional_light_color.a > 0.0) {
        return vec4<f32>(
            fog.base_color.rgb
                + scattering * fog.directional_light_color.rgb * fog.directional_light_color.a,
            fog.base_color.a,
        );
    } else {
        return fog.base_color;
    }
}

fn linear_fog(
    fog: Fog,
    input_color: vec4<f32>,
    distance: f32,
    scattering: vec3<f32>,
) -> vec4<f32> {
    var fog_color = scattering_adjusted_fog_color(fog, scattering);
    let start = fog.be.x;
    let end = fog.be.y;
    fog_color.a *= 1.0 - clamp((end - distance) / (end - start), 0.0, 1.0);
    return vec4<f32>(mix(input_color.rgb, fog_color.rgb, fog_color.a), input_color.a);
}

fn exponential_fog(
    fog: Fog,
    input_color: vec4<f32>,
    distance: f32,
    scattering: vec3<f32>,
) -> vec4<f32> {
    var fog_color = scattering_adjusted_fog_color(fog, scattering);
    let density = fog.be.x;
    fog_color.a *= 1.0 - 1.0 / exp(distance * density);
    return vec4<f32>(mix(input_color.rgb, fog_color.rgb, fog_color.a), input_color.a);
}

fn exponential_squared_fog(
    fog: Fog,
    input_color: vec4<f32>,
    distance: f32,
    scattering: vec3<f32>,
) -> vec4<f32> {
    var fog_color = scattering_adjusted_fog_color(fog, scattering);
    let distance_times_density = distance * fog.be.x;
    fog_color.a *= 1.0 - 1.0 / exp(distance_times_density * distance_times_density);
    return vec4<f32>(mix(input_color.rgb, fog_color.rgb, fog_color.a), input_color.a);
}

fn atmospheric_fog(
    fog: Fog,
    input_color: vec4<f32>,
    distance: f32,
    scattering: vec3<f32>,
) -> vec4<f32> {
    var fog_color = scattering_adjusted_fog_color(fog, scattering);
    let extinction_factor = 1.0 - 1.0 / exp(distance * fog.be);
    let inscattering_factor = 1.0 - 1.0 / exp(distance * fog.bi);
    return vec4<f32>(
        input_color.rgb * (1.0 - extinction_factor * fog_color.a)
            + fog_color.rgb * inscattering_factor * fog_color.a,
        input_color.a
    );
}

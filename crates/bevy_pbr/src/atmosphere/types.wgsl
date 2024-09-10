#define_import_path bevy_pbr::atmosphere::types

struct Atmosphere {
    // Radius of the planet
    bottom_radius: f32, //units: km

    // Radius at which we consider the atmosphere to 'end' for out calculations (from center of planet)
    top_radius: f32, //units: km

    rayleigh_density_exp_scale: f32,
    rayleigh_scattering: vec3<f32>,

    mie_density_exp_scale: f32,
    mie_scattering: f32, //units: km^-1
    mie_absorption: f32, //units: km^-1
    mie_phase_function_g: f32, //the "asymmetry" value of the phase function, unitless. Domain: (-1, 1)

    ozone_layer_center_altitude: f32, //units: km
    ozone_layer_half_width: f32, //units: km
    ozone_absorption: vec3<f32>, //ozone absorption. units: km^-1
};

struct AtmosphereLutSettings {
    transmittance_lut_size: vec2<f32>,
    multiscattering_lut_size: vec2<f32>,
    sky_view_lut_size: vec2<f32>,
    aerial_view_lut_size: vec3<f32>,
}

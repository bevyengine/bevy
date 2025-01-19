#define_import_path bevy_pbr::atmosphere::types

struct Atmosphere {
    // Radius of the planet
    bottom_radius: f32, // units: m

    // Radius at which we consider the atmosphere to 'end' for out calculations (from center of planet)
    top_radius: f32, // units: m

    ground_albedo: vec3<f32>,

    rayleigh_density_exp_scale: f32,
    rayleigh_scattering: vec3<f32>,

    mie_density_exp_scale: f32,
    mie_scattering: f32, // units: m^-1
    mie_absorption: f32, // units: m^-1
    mie_asymmetry: f32, // the "asymmetry" value of the phase function, unitless. Domain: (-1, 1)

    ozone_layer_altitude: f32, // units: m
    ozone_layer_width: f32, // units: m
    ozone_absorption: vec3<f32>, // ozone absorption. units: m^-1
}

struct AtmosphereSettings {
    transmittance_lut_size: vec2<u32>,
    multiscattering_lut_size: vec2<u32>,
    sky_view_lut_size: vec2<u32>,
    aerial_view_lut_size: vec3<u32>,
    transmittance_lut_samples: u32,
    multiscattering_lut_dirs: u32,
    multiscattering_lut_samples: u32,
    sky_view_lut_samples: u32,
    aerial_view_lut_samples: u32,
    aerial_view_lut_max_distance: f32,
    scene_units_to_m: f32,
}


// "Atmosphere space" is just the view position with y=0 and oriented horizontally,
// so the horizon stays a horizontal line in our luts
struct AtmosphereTransforms {
    world_from_atmosphere: mat4x4<f32>,
    atmosphere_from_world: mat4x4<f32>,
}

#define_import_path bevy_pbr::atmosphere::types

struct Atmosphere {
    ground_albedo: vec3<f32>,
    // Radius of the planet
    bottom_radius: f32, // units: m
    // Radius at which we consider the atmosphere to 'end' for out calculations (from center of planet)
    top_radius: f32, // units: m
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
    sky_max_samples: u32,
    rendering_method: u32,
}

// "Atmosphere space" uses local up for the zenith so the horizon-detail
// parameterization concentrates texels at the viewer's horizon. Azimuth uses a
// world-fixed reference so the terminator stays stable when tilting the camera.
struct AtmosphereTransforms {
    world_from_atmosphere: mat4x4<f32>,
    atmosphere_from_world: mat4x4<f32>,
}

struct AtmosphereData {
    atmosphere: Atmosphere,
    settings: AtmosphereSettings,
}
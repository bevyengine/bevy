#define_import_path bevy_pbr::sky_atmosphere

struct AtmosphereParameters {
  // Radius of the planet
  bottom_radius: f32,

  // Radius at which we consider the atmosphere to 'end' for out calculations (from center of planet)
  top_radius: f32,

  rayleigh_density_exp_scale: f32,
  rayleigh_scattering: vec3<f32>,

  mie_density_exp_scale: f32,
  mie_scattering: vec3<f32>,
  mie_extinction: vec3<f32>,
  mie_absorption: vec3<f32>,
  mie_phase_function_g: f32,

  ozone_density_layer_0_width: f32,
  ozone_density_layer_0_constant_term: f32,
  ozone_density_layer_0_linear_term: f32,
  ozone_density_layer_1_constant_term: f32,
  ozone_density_layer_1_linear_term: f32,

  absorption_extinction: vec3<f32>,

  ground_albedo: vec3<f32>,
};

fn get_atmosphere_parameters() -> AtmosphereParameters {
    var atmosphere: AtmosphereParameters;
    atmosphere.bottom_radius = 6360.0;
    atmosphere.top_radius = atmosphere.bottom_radius + 100.0;

	// Rayleigh scattering
    let earth_rayleigh_scale_height = 8.0;

    atmosphere.rayleigh_density_exp_scale = -1.0f / earth_rayleigh_scale_height;
    atmosphere.rayleigh_scattering = vec3<f32>(0.005802, 0.013558, 0.033100);

	// Mie scattering
    let earth_mie_scale_height = 1.2;

    atmosphere.mie_density_exp_scale = -1.0f / earth_mie_scale_height;
    atmosphere.mie_scattering = vec3<f32>(0.003996, 0.003996, 0.003996);
    atmosphere.mie_extinction = vec3<f32>(0.004440, 0.004440, 0.004440);
    atmosphere.mie_absorption = max(vec3(0.0), atmosphere.mie_extinction - atmosphere.mie_scattering);
    atmosphere.mie_phase_function_g = 0.8;

	// Ozone absorption
    atmosphere.ozone_density_layer_0_width = 25.0;
    atmosphere.ozone_density_layer_0_constant_term = -2.0 / 3.0;
    atmosphere.ozone_density_layer_0_linear_term = 1.0 / 15.0;
    atmosphere.ozone_density_layer_1_constant_term = 8.0 / 3.0;
    atmosphere.ozone_density_layer_1_linear_term = -1.0 / 15.0;

    atmosphere.absorption_extinction = vec3<f32>(0.000650, 0.001881, 0.000085);

    atmosphere.ground_albedo = vec3<f32>(0.3f);

    return atmosphere;
}

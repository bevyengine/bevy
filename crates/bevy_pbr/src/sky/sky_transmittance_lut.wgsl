#import bevy_pbr::{
    sky_atmosphere::{AtmosphereParameters, get_atmosphere_parameters},
    sky_common::{uv_to_r_mu, distance_to_top_atmosphere_boundary, distance_to_bottom_atmosphere_boundary}
}

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@fragment
fn main(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
  let atmosphere = get_atmosphere_parameters();

  // map UV coordinates to view height (r) and zenith cos angle (mu)
  let r_mu = uv_to_r_mu(atmosphere, in.uv);

  // compute the optical depth from view height r to the top atmosphere boundary
  let optical_depth = compute_optical_depth_to_top_atmosphere_boundary(atmosphere, r_mu.x, r_mu.y);

  let transmittance = exp(-optical_depth);

  return vec4<f32>(transmittance, 1.0);
}

/// Compute the optical depth of the atmosphere from the ground to the top atmosphere boundary
/// at a given view height (r) and zenith cos angle (mu)
fn compute_optical_depth_to_top_atmosphere_boundary(atmosphere: AtmosphereParameters, r: f32, mu:f32) -> vec3<f32> {
  let t_bottom = distance_to_bottom_atmosphere_boundary(atmosphere, r, mu);
  let t_top = distance_to_top_atmosphere_boundary(atmosphere, r, mu);
  let t_max = max(t_bottom, t_top);

  let sample_count = 40u;

  var optical_depth = vec3<f32>(0.0f);
  var prev_t = 0.0f;

  for (var i = 0u; i < sample_count; i++) {
    // SebH uses this for multiple scattering. It might not be needed here, but I've kept it to get results that are as close as possible to the original
    let t_i = (t_max * f32(i) + 0.3f) / f32(sample_count);
    let dt = t_i - prev_t;
    prev_t = t_i;

    // distance r from current sample point to planet center
    let r_i = sqrt(t_i * t_i + 2.0 * r * mu * t_i + r * r);
    let view_height = r_i - atmosphere.bottom_radius;

    let atmosphere_sample = sample_atmosphere(atmosphere, view_height);
    let sample_optical_depth = atmosphere_sample.extinction * dt;

    optical_depth += sample_optical_depth;
  }

  return optical_depth;
}

struct AtmosphereSample {
  scattering: vec3<f32>,
  absorption: vec3<f32>,
  extinction: vec3<f32>
};

fn sample_atmosphere(atmosphere: AtmosphereParameters, view_height: f32) -> AtmosphereSample {
  var result: AtmosphereSample;

  // atmosphere values at view_height
  let mie_density = exp(atmosphere.mie_density_exp_scale * view_height);
  let rayleigh_density = exp(atmosphere.rayleigh_density_exp_scale * view_height);
  var ozone_density: f32;
  if (view_height < atmosphere.ozone_density_layer_0_width) {
    ozone_density = atmosphere.ozone_density_layer_0_linear_term * view_height + atmosphere.ozone_density_layer_0_constant_term;
  } else {
    ozone_density = atmosphere.ozone_density_layer_1_linear_term * view_height + atmosphere.ozone_density_layer_1_constant_term;
  }
  ozone_density = saturate(ozone_density);

  let mie_scattering = mie_density * atmosphere.mie_scattering;
  let mie_absorption = mie_density * atmosphere.mie_absorption;
  let mie_extinction = mie_density * atmosphere.mie_extinction;

  let rayleigh_scattering = rayleigh_density * atmosphere.rayleigh_scattering;
  // no rayleigh absorption
  // rayleigh extinction is the sum of scattering and absorption

  // ozone doesn't contribute to scattering
  let ozone_absorption = ozone_density * atmosphere.absorption_extinction;
  // ozone extinction is the sum of scattering and absorption

  result.scattering = mie_scattering + rayleigh_scattering;
  result.absorption = mie_absorption + ozone_absorption;
  result.extinction = mie_extinction + rayleigh_scattering + ozone_absorption;

  return result;
}


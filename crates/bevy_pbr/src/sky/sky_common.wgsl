#define_import_path bevy_pbr::sky_common

#import bevy_pbr::{
    sky_atmosphere::AtmosphereParameters,
}

// Mapping from view height (r) and zenith cos angle (mu) to UV coordinates in the transmittance LUT
// Assuming r between ground and top atmosphere boundary, and mu = cos(zenith_angle)
// Chosen to increase precision near the ground and to work around a discontinuity at the horizon
// See Bruneton and Neyret 2008, "Precomputed Atmospheric Scattering" section 4
fn r_mu_to_uv(atmosphere: AtmosphereParameters, r: f32, mu: f32) -> vec2<f32> {
  // Distance along a horizontal ray from the ground to the top atmosphere boundary
  let H = sqrt(atmosphere.top_radius * atmosphere.top_radius -
      atmosphere.bottom_radius * atmosphere.bottom_radius);

  // Distance from a point at height r to the horizon
  // ignore the case where r <= atmosphere.bottom_radius
  let rho = sqrt(max(r * r - atmosphere.bottom_radius * atmosphere.bottom_radius, 0.0));

  // Distance from a point at height r to the top atmosphere boundary at zenith angle mu
  let d = distance_to_top_atmosphere_boundary(atmosphere, r, mu);

  // Minimum and maximum distance to the top atmosphere boundary from a point at height r
  let d_min = atmosphere.top_radius - r; // length of the ray straight up to the top atmosphere boundary
  let d_max = rho + H; // length of the ray to the top atmosphere boundary and grazing the horizon

  let u = (d - d_min) / (d_max - d_min);
  let v = rho / H;
  return vec2<f32>(u, v);
}

// Inverse of the mapping above, mapping from UV coordinates in the transmittance LUT to view height (r) and zenith cos angle (mu)
fn uv_to_r_mu(atmosphere: AtmosphereParameters, uv: vec2<f32>) -> vec2<f32> {
  // Distance to top atmosphere boundary for a horizontal ray at ground level
  let H = sqrt(atmosphere.top_radius * atmosphere.top_radius - atmosphere.bottom_radius * atmosphere.bottom_radius);

  // Distance to the horizon, from which we can compute r:
  let rho = H * uv.y;
  let r = sqrt(rho * rho + atmosphere.bottom_radius * atmosphere.bottom_radius);

  // Distance to the top atmosphere boundary for the ray (r,mu), and its minimum
  // and maximum values over all mu - obtained for (r,1) and (r,mu_horizon) -
  // from which we can recover mu:
  let d_min = atmosphere.top_radius - r;
  let d_max = rho + H;
  let d = d_min + uv.x * (d_max - d_min);

  var mu: f32;
  if (d == 0.0) {
    mu = 1.0;
  } else {
    mu = (H * H - rho * rho - d * d) / (2.0 * r * d);
  }

  mu = clamp(mu, -1.0, 1.0);

  return vec2<f32>(r, mu);
}

/// Simplified ray-sphere intersection
/// where:
/// Ray origin, o = [0,0,r] with r <= atmosphere.top_radius
/// mu is the cosine of spherical coordinate theta (-1.0 <= mu <= 1.0)
/// so ray direction in spherical coordinates is [1,acos(mu),0] which needs to be converted to cartesian
/// Direction of ray, u = [0,sqrt(1-mu*mu),mu]
/// Center of sphere, c = [0,0,0]
/// Radius of sphere, r = atmosphere.top_radius
/// This function solves the quadratic equation for line-sphere intersection simplified under these assumptions
fn distance_to_top_atmosphere_boundary(atmosphere: AtmosphereParameters, r: f32, mu: f32) -> f32 {
  // ignore the case where r > atmosphere.top_radius
  let positive_discriminant = max(r * r * (mu * mu - 1.0) + atmosphere.top_radius * atmosphere.top_radius, 0.0);
  return max(-r * mu + sqrt(positive_discriminant), 0.0);
}

/// Simplified ray-sphere intersection
/// as above for intersections with the ground
fn distance_to_bottom_atmosphere_boundary(atmosphere: AtmosphereParameters, r: f32, mu: f32) -> f32 {
    let positive_discriminant = max(r * r * (mu * mu - 1.0) + atmosphere.bottom_radius * atmosphere.bottom_radius, 0.0);
    return max(-r * mu - sqrt(positive_discriminant), 0.0);
}


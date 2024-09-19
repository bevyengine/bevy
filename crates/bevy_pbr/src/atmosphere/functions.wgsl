#define_import_path bevy_pbr::atmosphere::functions

#import bevy_pbr::atmosphere::types::Atmosphere,


// Mapping from view height (r) and zenith cos angle (mu) to UV coordinates in the transmittance LUT
// Assuming r between ground and top atmosphere boundary, and mu = cos(zenith_angle)
// Chosen to increase precision near the ground and to work around a discontinuity at the horizon
// See Bruneton and Neyret 2008, "Precomputed Atmospheric Scattering" section 4
fn transmittance_lut_r_mu_to_uv(atmosphere: Atmosphere, r: f32, mu: f32) -> vec2<f32> {
  // Distance along a horizontal ray from the ground to the top atmosphere boundary
    let H = sqrt(atmosphere.top_radius * atmosphere.top_radius - atmosphere.bottom_radius * atmosphere.bottom_radius);

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
fn transmittance_lut_uv_to_r_mu(atmosphere: Atmosphere, uv: vec2<f32>) -> vec2<f32> {
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
    if d == 0.0 {
        mu = 1.0;
    } else {
        mu = (H * H - rho * rho - d * d) / (2.0 * r * d);
    }

    mu = clamp(mu, -1.0, 1.0);

    return vec2<f32>(r, mu);
}

fn multiscattering_lut_r_mu_to_uv(atmosphere: Atmosphere, r_mu: vec3<f32>) -> vec3<f32> {
}

fn multiscattering_lut_uv_to_r_mu(atmosphere: Atmosphere, uv: vec2<f32>) -> vec2<f32> {
}

fn sample_transmittance_lut(atmosphere: Atmosphere, lut: texture_2d<f32>, smp: sampler, position: vec3<f32>, dir: vec3<f32>) -> vec3<f32> {
    let r = position.y;
    let mu = dir_to_light.y;
    let uv = transmittance_lut_r_mu_to_uv(atmosphere, r, mu);
    return textureSample(smp, lut).rgb;
}

fn sample_multiscattering_lut(atmosphere: Atmosphere, lut: texture_3d<f32>, smp: sampler, position: vec3<f32>, dir: vec3<f32>) -> vec3<f32> {
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
fn distance_to_top_atmosphere_boundary(atmosphere: Atmosphere, r: f32, mu: f32) -> f32 {
  // ignore the case where r > atmosphere.top_radius
    let positive_discriminant = max(r * r * (mu * mu - 1.0) + atmosphere.top_radius * atmosphere.top_radius, 0.0);
    return max(-r * mu + sqrt(positive_discriminant), 0.0);
}

/// Simplified ray-sphere intersection
/// as above for intersections with the ground
fn distance_to_bottom_atmosphere_boundary(atmosphere: Atmosphere, r: f32, mu: f32) -> f32 {
    let positive_discriminant = max(r * r * (mu * mu - 1.0) + atmosphere.bottom_radius * atmosphere.bottom_radius, 0.0);
    return max(-r * mu - sqrt(positive_discriminant), 0.0);
}

struct AtmosphereSample {
    scattering: vec3<f32>,
    absorption: vec3<f32>,
    extinction: vec3<f32>
};

//prob fine to return big struct because of inlining
fn sample_atmosphere(atmosphere: Atmosphere, view_height: f32) -> AtmosphereSample {
    var result: AtmosphereSample;

    // atmosphere values at view_height
    let mie_density = exp(atmosphere.mie_density_exp_scale * view_height); //TODO: zero-out when above atmosphere boundary? i mean the raycast will stop anyway
    let rayleigh_density = exp(atmosphere.rayleigh_density_exp_scale * view_height);
    var ozone_density: f32 = max(0.0, 1.0 - (abs(view_height - atmosphere.ozone_layer_center_altitude) / atmosphere.ozone_layer_half_width));

    let mie_scattering = mie_density * atmosphere.mie_scattering;
    let mie_absorption = mie_density * atmosphere.mie_absorption;
    let mie_extinction = mie_scattering + mie_absorption;

    let rayleigh_scattering = rayleigh_density * atmosphere.rayleigh_scattering;
    // no rayleigh absorption
    // rayleigh extinction is the sum of scattering and absorption

    // ozone doesn't contribute to scattering
    let ozone_absorption = ozone_density * atmosphere.ozone_absorption;
    // ozone extinction is the sum of scattering and absorption

    result.scattering = mie_scattering + rayleigh_scattering;
    result.absorption = mie_absorption + ozone_absorption;
    result.extinction = mie_extinction + rayleigh_scattering + ozone_absorption;

    return result;
}

// 3 / (16π)
const FRAC_3_16_PI: f32 = 0.0596831036594607509;

// 1 / (4π)
const FRAC_4_PI: f32 = 0.07957747154594767;

fn rayleigh(neg_LdotV: f32) -> f32 {
    FRAC_3_16_PI * (1 + (neg_LdotV * neg_LdotV));
}

fn henyey_greenstein(neg_LdotV: f32) -> f32 {
    let g = volumetric_fog.scattering_asymmetry;
    let denom = 1.0 + g * g - 2.0 * g * neg_LdotV;
    return FRAC_4_PI * (1.0 - g * g) / (denom * sqrt(denom));
}

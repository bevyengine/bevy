#define_import_path bevy_pbr::atmosphere::functions

#import bevy_pbr::atmosphere::{
    types::Atmosphere,
    bindings::{atmosphere, settings, view, lights, transmittance_lut, transmittance_lut_sampler, multiscattering_lut, multiscattering_lut_sampler}
    bruneton_functions::{transmittance_lut_r_mu_to_uv, transmittance_lut_uv_to_r_mu, ray_intersects_ground},
}

// CONSTANTS

const PI: f32 = 3.141592653589793238462;
const TAU: f32 = 6.283185307179586476925;
const FRAC_PI: f32 = 0.31830988618379067153; // 1 / π
const FRAC_3_16_PI: f32 = 0.0596831036594607509; // 3 / (16π)
const FRAC_4_PI: f32 = 0.07957747154594767; // 1 / (4π)

// LUT UV PARAMATERIZATIONS

fn multiscattering_lut_r_mu_to_uv(altitude: f32, cos_azimuth: f32) -> vec2<f32> {
    let u = 0.5 + 0.5 * cos_azimuth;
    let v = saturate(altitude / (atmosphere.top_radius - atmosphere.bottom_radius));
    return vec2(u, v);
}

fn multiscattering_lut_uv_to_r_mu(uv: vec2<f32>) -> vec2<f32> {
    let altitude = (atmosphere.top_radius - atmosphere.bottom_radius) * uv.y;
    let cos_azimuth = uv.x * 2 - 1;
    return vec2(altitude, cos_azimuth);
}

fn sky_view_lut_lat_long_to_uv(lat: f32, long: f32) -> vec2<f32> {
    let u = long * FRAC_PI + 0.5;
    let v = sqrt(2 * abs(lat) * FRAC_PI) * sign(lat) * 0.5 + 0.5;
    return vec2(u, v);
}

fn sky_view_lut_uv_to_lat_long(uv: vec2<f32>) -> vec2<f32> {
    let long = (uv.x - 0.5) * PI;
    let v_minus_half = uv.y - 0.5;
    let lat = TAU * (v_minus_half * v_minus_half) * sign(v_minus_half);
    return vec2(lat, long);
}

// LUT SAMPLING

fn sample_transmittance_lut(altitude: f32, cos_azimuth: f32) -> vec3<f32> {
    let uv = transmittance_lut_r_mu_to_uv(altitude, cos_azimuth);
    return textureSampleLevel(transmittance_lut, transmittance_lut_sampler, uv, 0.0).rgb;
}

fn sample_multiscattering_lut(altitude: f32, cos_azimuth: f32) -> vec3<f32> {
    let uv = multiscattering_lut_r_mu_to_uv(altitude, cos_azimuth);
    return textureSampleLevel(multiscattering_lut, multiscattering_lut_sampler, uv, 0.0).rgb;
}

// PHASE FUNCTIONS

fn rayleigh(neg_LdotV: f32) -> f32 {
    return FRAC_3_16_PI * (1 + (neg_LdotV * neg_LdotV));
}

fn henyey_greenstein(neg_LdotV: f32, g: f32) -> f32 {
    let denom = 1.0 + g * g - 2.0 * g * neg_LdotV;
    return FRAC_4_PI * (1.0 - g * g) / (denom * sqrt(denom));
}


// ATMOSPHERE SAMPLING

struct AtmosphereSample {
    rayleigh_scattering: vec3<f32>,
    mie_scattering: f32,
    extinction: vec3<f32>
}

//prob fine to return big struct because of inlining
fn sample_atmosphere(altitude: f32) -> AtmosphereSample {

    // atmosphere values at altitude
    let mie_density = exp(atmosphere.mie_density_exp_scale * altitude); //TODO: zero-out when above atmosphere boundary? i mean the raycast will stop anyway
    let rayleigh_density = exp(atmosphere.rayleigh_density_exp_scale * altitude);
    var ozone_density: f32 = max(0.0, 1.0 - (abs(altitude - atmosphere.ozone_layer_center_altitude) / atmosphere.ozone_layer_half_width));

    let mie_scattering = mie_density * atmosphere.mie_scattering;
    let mie_absorption = mie_density * atmosphere.mie_absorption;
    let mie_extinction = mie_scattering + mie_absorption;

    let rayleigh_scattering = rayleigh_density * atmosphere.rayleigh_scattering;
    // no rayleigh absorption
    // rayleigh extinction is the sum of scattering and absorption

    // ozone doesn't contribute to scattering
    let ozone_absorption = ozone_density * atmosphere.ozone_absorption;

    var sample: AtmosphereSample;
    sample.rayleigh_scattering = rayleigh_scattering;
    sample.mie_scattering = mie_scattering;
    sample.extinction = rayleigh_scattering + mie_extinction + ozone_absorption;

    return sample;
}

fn sample_local_inscattering(local_atmosphere: AtmosphereSample, transmittance_to_sample: vec3<f32>, view_dir: vec3<f32>, altitude: f32) -> vec3<f32> {
    //TODO: storing these outside the loop saves several multiplications, but at the cost of an extra vector register
    var rayleigh_scattering = vec3(0.0);
    var mie_scattering = vec3(0.0);
    for (var light_i: u32 = 0u; light_i < lights.n_directional_lights; light_i++) {
        let light = &lights.directional_lights[light_i];
        let light_cos_azimuth = (*light).direction_to_light.y;
        let neg_LdotV = dot(view_dir, (*light).direction_to_light);
        let rayleigh_phase = rayleigh(neg_LdotV);
        let mie_phase = henyey_greenstein(neg_LdotV, atmosphere.mie_asymmetry);

        let transmittance_to_light = sample_transmittance_lut(altitude, light_cos_azimuth);
        let shadow_factor = transmittance_to_light * f32(!ray_intersects_ground(altitude, light_cos_azimuth));

        let psi_ms = sample_multiscattering_lut(altitude, light_cos_azimuth);

        rayleigh_scattering += (transmittance_to_sample * shadow_factor * rayleigh_phase + psi_ms) * (*light).color.rgb;
        mie_scattering += (transmittance_to_sample * shadow_factor * mie_phase + psi_ms) * (*light).color.rgb;
    }
    return local_atmosphere.rayleigh_scattering * rayleigh_scattering + local_atmosphere.mie_scattering * mie_scattering;
}


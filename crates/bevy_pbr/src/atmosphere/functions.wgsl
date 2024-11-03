#define_import_path bevy_pbr::atmosphere::functions

#import bevy_pbr::atmosphere::{
    types::Atmosphere,
    bindings::{
        atmosphere, settings, view, lights, transmittance_lut, transmittance_lut_sampler, 
        multiscattering_lut, multiscattering_lut_sampler, sky_view_lut, sky_view_lut_sampler,
        aerial_view_lut, aerial_view_lut_sampler
    },
    bruneton_functions::{transmittance_lut_r_mu_to_uv, transmittance_lut_uv_to_r_mu, ray_intersects_ground},
}

// CONSTANTS

const PI: f32 = 3.141592653589793238462;
const TAU: f32 = 6.283185307179586476925;
const FRAC_PI: f32 = 0.31830988618379067153; // 1 / π
const FRAC_3_16_PI: f32 = 0.0596831036594607509; // 3 / (16π)
const FRAC_4_PI: f32 = 0.07957747154594767; // 1 / (4π)

// LUT UV PARAMATERIZATIONS

fn multiscattering_lut_r_mu_to_uv(r: f32, mu: f32) -> vec2<f32> {
    let u = 0.5 + 0.5 * mu;
    let v = saturate((r - atmosphere.bottom_radius) / (atmosphere.top_radius - atmosphere.bottom_radius)); //TODO
    return vec2(u, v);
}

fn multiscattering_lut_uv_to_r_mu(uv: vec2<f32>) -> vec2<f32> {
    let r = mix(atmosphere.bottom_radius, atmosphere.top_radius, uv.y);
    let mu = uv.x * 2 - 1;
    return vec2(r, mu);
}

fn sky_view_lut_lat_long_to_uv(lat: f32, long: f32) -> vec2<f32> {
    let u = long * FRAC_PI + 0.5;
    let v = sqrt(2 * abs(lat) * FRAC_PI) * sign(lat) * 0.5 + 0.5;
    return vec2(u, v);
}

fn sky_view_lut_uv_to_lat_long(uv: vec2<f32>) -> vec2<f32> {
    let long = (uv.x - 0.5) * TAU;
    let v_minus_half = uv.y - 0.5;
    let lat = TAU * (v_minus_half * v_minus_half) * sign(v_minus_half);
    return vec2(lat, long);
}

// LUT SAMPLING

fn sample_transmittance_lut(r: f32, mu: f32) -> vec3<f32> {
    let uv = transmittance_lut_r_mu_to_uv(r, mu);
    return textureSampleLevel(transmittance_lut, transmittance_lut_sampler, uv, 0.0).rgb;
}

fn sample_multiscattering_lut(r: f32, mu: f32) -> vec3<f32> {
    let uv = multiscattering_lut_r_mu_to_uv(r, mu);
    return textureSampleLevel(multiscattering_lut, multiscattering_lut_sampler, uv, 0.0).rgb;
}

fn sample_sky_view_lut(ray_dir: vec3<f32>) -> vec3<f32> {
    let lat_long = ray_dir_to_lat_long(ray_dir);
    let uv = sky_view_lut_lat_long_to_uv(lat_long.x, lat_long.y);
    return textureSampleLevel(sky_view_lut, sky_view_lut_sampler, uv, 0.0).rgb;
}

//RGB channels: total inscattered light along the camera ray to the current sample.
//A channel: average transmittance across all wavelengths to the current sample.
fn sample_aerial_view_lut(ndc: vec3<f32>) -> vec4<f32> {
    return textureSampleLevel(aerial_view_lut, aerial_view_lut_sampler, ndc, 0.0);
}

// PHASE FUNCTIONS

fn rayleigh(neg_LdotV: f32) -> f32 {
    return FRAC_3_16_PI * (1 + (neg_LdotV * neg_LdotV));
}

fn henyey_greenstein(neg_LdotV: f32) -> f32 {
    let g = atmosphere.mie_asymmetry;
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
fn sample_atmosphere(r: f32) -> AtmosphereSample {
    let altitude = r - atmosphere.bottom_radius;

    // atmosphere values at altitude
    let mie_density = exp(atmosphere.mie_density_exp_scale * altitude);
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

fn sample_local_inscattering(local_atmosphere: AtmosphereSample, transmittance_to_sample: vec3<f32>, view_dir: vec3<f32>, local_r: f32, local_up: vec3<f32>) -> vec3<f32> {
    //TODO: storing these outside the loop saves several multiplications, but at the cost of an extra vector register
    var rayleigh_scattering = vec3(0.0);
    var mie_scattering = vec3(0.0);
    for (var light_i: u32 = 0u; light_i < lights.n_directional_lights; light_i++) {
        let light = &lights.directional_lights[light_i];
        let mu_light = dot((*light).direction_to_light, local_up);
        let neg_LdotV = dot((*light).direction_to_light, view_dir);
        let rayleigh_phase = rayleigh(neg_LdotV);
        let mie_phase = henyey_greenstein(neg_LdotV);

        let transmittance_to_light = sample_transmittance_lut(local_r, mu_light);
        let shadow_factor = transmittance_to_light * f32(!ray_intersects_ground(local_r, mu_light));

        let psi_ms = sample_multiscattering_lut(local_r, mu_light);

        rayleigh_scattering += (transmittance_to_sample * shadow_factor * rayleigh_phase + psi_ms) * (*light).color.rgb;
        mie_scattering += (transmittance_to_sample * shadow_factor * mie_phase + psi_ms) * (*light).color.rgb;
    }
    return local_atmosphere.rayleigh_scattering * rayleigh_scattering + local_atmosphere.mie_scattering * mie_scattering;
}

// TRANSFORM UTILITIES

//We assume the `up` vector at the view position is the y axis, since the world is locally flat/level.
//NOTE: this means that if your bevy world is actually placed on a sphere, this will be wrong.
fn get_local_up(view_r: f32, displacement: vec3<f32>) -> vec3<f32> {
    return normalize(vec3(0.0, view_r, 0.0) + displacement);
}

fn get_local_r(view_r: f32, view_mu: f32, dist: f32) -> f32 {
    return sqrt(dist * dist + 2.0 * view_r * view_mu * dist + view_r * view_r);
}

// Convert uv [0.0 .. 1.0] coordinate to ndc space xy [-1.0 .. 1.0]
fn uv_to_ndc(uv: vec2<f32>) -> vec2<f32> {
    return uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0);
}

/// Convert a ndc space position to world space
fn position_ndc_to_world(ndc_pos: vec3<f32>) -> vec3<f32> {
    let world_pos = view.world_from_clip * vec4(ndc_pos, 1.0);
    return world_pos.xyz / world_pos.w;
}

//Modified from skybox.wgsl. For this pass we don't need to apply a separate sky transform or consider camera viewport.
//w component is the cosine of the view direction with the view forward vector, to correct step distance at the edges of the viewport
fn uv_to_ray_direction(uv: vec2<f32>) -> vec4<f32> {
    // Using world positions of the fragment and camera to calculate a ray direction
    // breaks down at large translations. This code only needs to know the ray direction.
    // The ray direction is along the direction from the camera to the fragment position.
    // In view space, the camera is at the origin, so the view space ray direction is
    // along the direction of the fragment position - (0,0,0) which is just the
    // fragment position.
    // Use the position on the near clipping plane to avoid -inf world position
    // because the far plane of an infinite reverse projection is at infinity.
    let view_position_homogeneous = view.view_from_clip * vec4(
        uv_to_ndc(uv),
        1.0,
        1.0,
    );

    // Transforming the view space ray direction by the skybox transform matrix, it is 
    // equivalent to rotating the skybox itself.
    let view_ray_direction = view_position_homogeneous.xyz / view_position_homogeneous.w; //TODO: remove this step and just use position_ndc_to_world? we didn't need to transform in view space

    // Transforming the view space ray direction by the inverse view matrix, transforms the
    // direction to world space. Note that the w element is set to 0.0, as this is a
    // vector direction, not a position, That causes the matrix multiplication to ignore
    // the translations from the view matrix.
    let ray_direction = (view.world_from_view * vec4(view_ray_direction, 0.0)).xyz;

    return vec4(normalize(ray_direction), -view_ray_direction.z);
}

fn ray_dir_to_lat_long(ray_dir: vec3<f32>) -> vec2<f32> {
    let view_dir = -view.world_from_view[2].xyz;
    let lat = asin(ray_dir.y);
    let long = atan2(view_dir.x * ray_dir.z - view_dir.z * ray_dir.x, view_dir.x * ray_dir.x + view_dir.z + ray_dir.z); //TODO: explain
    return vec2(lat, long);
}

/// Convert ndc depth to linear view z. 
/// Note: Depth values in front of the camera will be negative as -z is forward
fn depth_ndc_to_view_z(ndc_depth: f32) -> f32 {
    let view_pos = view.view_from_clip * vec4(0.0, 0.0, ndc_depth, 1.0);
    return view_pos.z / view_pos.w;
}

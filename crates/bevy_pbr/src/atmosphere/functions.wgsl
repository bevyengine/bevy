#define_import_path bevy_pbr::atmosphere::functions

#import bevy_render::maths::{PI, HALF_PI, PI_2, fast_acos, fast_acos_4, fast_atan2, ray_sphere_intersect}

#import bevy_pbr::atmosphere::{
    types::Atmosphere,
    bindings::{
        atmosphere, settings, view, lights, transmittance_lut, transmittance_lut_sampler, 
        multiscattering_lut, multiscattering_lut_sampler, sky_view_lut, sky_view_lut_sampler,
        aerial_view_lut, aerial_view_lut_sampler, atmosphere_transforms
    },
    bruneton_functions::{
        transmittance_lut_r_mu_to_uv, transmittance_lut_uv_to_r_mu, 
        ray_intersects_ground, distance_to_top_atmosphere_boundary, 
        distance_to_bottom_atmosphere_boundary
    },
}

// NOTE FOR CONVENTIONS: 
// r:
//   radius, or distance from planet center 
//
// altitude:
//   distance from planet **surface**
//
// mu:
//   cosine of the zenith angle of a ray with
//   respect to the planet normal
//
// atmosphere space:
//   abbreviated as "as" (contrast with vs, cs, ws), this space is similar
//   to view space, but with the camera positioned horizontally on the planet
//   surface, so the horizon is a horizontal line centered vertically in the
//   frame. This enables the non-linear latitude parametrization the paper uses 
//   to concentrate detail near the horizon 


// CONSTANTS
const FRAC_PI: f32 = 0.3183098862; // 1 / π
const FRAC_2_PI: f32 = 0.15915494309;  // 1 / (2π)
const FRAC_3_16_PI: f32 = 0.0596831036594607509; // 3 / (16π)
const FRAC_4_PI: f32 = 0.07957747154594767; // 1 / (4π)
const ROOT_2: f32 = 1.41421356; // √2
const EPSILON: f32 = 1.0; // 1 meter

// During raymarching, each segment is sampled at a single point. This constant determines
// where in the segment that sample is taken (0.0 = start, 0.5 = middle, 1.0 = end).
// We use 0.3 to sample closer to the start of each segment, which better approximates
// the exponential falloff of atmospheric density.
const MIDPOINT_RATIO: f32 = 0.3;

// LUT UV PARAMETERIZATIONS

fn unit_to_sub_uvs(val: vec2<f32>, resolution: vec2<f32>) -> vec2<f32> {
    return (val + 0.5f / resolution) * (resolution / (resolution + 1.0f));
}

fn sub_uvs_to_unit(val: vec2<f32>, resolution: vec2<f32>) -> vec2<f32> {
    return (val - 0.5f / resolution) * (resolution / (resolution - 1.0f));
}

fn multiscattering_lut_r_mu_to_uv(r: f32, mu: f32) -> vec2<f32> {
    let u = 0.5 + 0.5 * mu;
    let v = saturate((r - atmosphere.bottom_radius) / (atmosphere.top_radius - atmosphere.bottom_radius)); //TODO
    return unit_to_sub_uvs(vec2(u, v), vec2<f32>(settings.multiscattering_lut_size));
}

fn multiscattering_lut_uv_to_r_mu(uv: vec2<f32>) -> vec2<f32> {
    let adj_uv = sub_uvs_to_unit(uv, vec2<f32>(settings.multiscattering_lut_size));
    let r = mix(atmosphere.bottom_radius, atmosphere.top_radius, adj_uv.y);
    let mu = adj_uv.x * 2 - 1;
    return vec2(r, mu);
}

fn sky_view_lut_r_mu_azimuth_to_uv(r: f32, mu: f32, azimuth: f32) -> vec2<f32> {
    let u = (azimuth * FRAC_2_PI) + 0.5;

    let v_horizon = sqrt(r * r - atmosphere.bottom_radius * atmosphere.bottom_radius);
    let cos_beta = v_horizon / r;
    // Using fast_acos_4 for better precision at small angles
    // to avoid artifacts at the horizon
    let beta = fast_acos_4(cos_beta);
    let horizon_zenith = PI - beta;
    let view_zenith = fast_acos_4(mu);

    // Apply non-linear transformation to compress more texels 
    // near the horizon where high-frequency details matter most
    // l is latitude in [-π/2, π/2] and v is texture coordinate in [0,1]
    let l = view_zenith - horizon_zenith;
    let abs_l = abs(l);

    let v = 0.5 + 0.5 * sign(l) * sqrt(abs_l / HALF_PI);

    return unit_to_sub_uvs(vec2(u, v), vec2<f32>(settings.sky_view_lut_size));
}

fn sky_view_lut_uv_to_zenith_azimuth(r: f32, uv: vec2<f32>) -> vec2<f32> {
    let adj_uv = sub_uvs_to_unit(vec2(uv.x, 1.0 - uv.y), vec2<f32>(settings.sky_view_lut_size));
    let azimuth = (adj_uv.x - 0.5) * PI_2;

    // Horizon parameters
    let v_horizon = sqrt(r * r - atmosphere.bottom_radius * atmosphere.bottom_radius);
    let cos_beta = v_horizon / r;
    let beta = fast_acos_4(cos_beta);
    let horizon_zenith = PI - beta;

    // Inverse of horizon-detail mapping to recover original latitude from texture coordinate
    let t = abs(2.0 * (adj_uv.y - 0.5));
    let l = sign(adj_uv.y - 0.5) * HALF_PI * t * t;

    return vec2(horizon_zenith - l, azimuth);
}

// LUT SAMPLING

fn sample_transmittance_lut(r: f32, mu: f32) -> vec3<f32> {
    let uv = transmittance_lut_r_mu_to_uv(r, mu);
    return textureSampleLevel(transmittance_lut, transmittance_lut_sampler, uv, 0.0).rgb;
}

// NOTICE: This function is copyrighted by Eric Bruneton and INRIA, and falls
// under the license reproduced in bruneton_functions.wgsl (variant of MIT license)
//
// FIXME: this function should be in bruneton_functions.wgsl, but because naga_oil doesn't 
// support cyclic imports it's stuck here
fn sample_transmittance_lut_segment(r: f32, mu: f32, t: f32) -> vec3<f32> {
    let r_t = get_local_r(r, mu, t);
    let mu_t = clamp((r * mu + t) / r_t, -1.0, 1.0);

    if ray_intersects_ground(r, mu) {
        return min(
            sample_transmittance_lut(r_t, -mu_t) / sample_transmittance_lut(r, -mu),
            vec3(1.0)
        );
    } else {
        return min(
            sample_transmittance_lut(r, mu) / sample_transmittance_lut(r_t, mu_t), vec3(1.0)
        );
    }
}

fn sample_multiscattering_lut(r: f32, mu: f32) -> vec3<f32> {
    let uv = multiscattering_lut_r_mu_to_uv(r, mu);
    return textureSampleLevel(multiscattering_lut, multiscattering_lut_sampler, uv, 0.0).rgb;
}

fn sample_sky_view_lut(r: f32, ray_dir_as: vec3<f32>) -> vec3<f32> {
    let mu = ray_dir_as.y;
    let azimuth = fast_atan2(ray_dir_as.x, -ray_dir_as.z);
    let uv = sky_view_lut_r_mu_azimuth_to_uv(r, mu, azimuth);
    return textureSampleLevel(sky_view_lut, sky_view_lut_sampler, uv, 0.0).rgb;
}

fn ndc_to_camera_dist(ndc: vec3<f32>) -> f32 {
    let view_pos = view.view_from_clip * vec4(ndc, 1.0);
    let t = length(view_pos.xyz / view_pos.w) * settings.scene_units_to_m;
    return t;
}

// RGB channels: total inscattered light along the camera ray to the current sample.
// A channel: average transmittance across all wavelengths to the current sample.
fn sample_aerial_view_lut(uv: vec2<f32>, t: f32) -> vec3<f32> {
    let t_max = settings.aerial_view_lut_max_distance;
    let num_slices = f32(settings.aerial_view_lut_size.z);
    // Each texel stores the value of the scattering integral over the whole slice,
    // which requires us to offset the w coordinate by half a slice. For
    // example, if we wanted the value of the integral at the boundary between slices,
    // we'd need to sample at the center of the previous slice, and vice-versa for
    // sampling in the center of a slice.
    let uvw = vec3(uv, saturate(t / t_max - 0.5 / num_slices));
    let sample = textureSampleLevel(aerial_view_lut, aerial_view_lut_sampler, uvw, 0.0);
    // Since sampling anywhere between w=0 and w=t_slice will clamp to the first slice,
    // we need to do a linear step over the first slice towards zero at the camera's
    // position to recover the correct integral value.
    let t_slice = t_max / num_slices;
    let fade = saturate(t / t_slice);
    // Recover the values from log space
    return exp(sample.rgb) * fade;
}

// PHASE FUNCTIONS

// -(L . V) == (L . -V). -V here is our ray direction, which points away from the view 
// instead of towards it (which would be the *view direction*, V)

// evaluates the rayleigh phase function, which describes the likelihood
// of a rayleigh scattering event scattering light from the light direction towards the view
fn rayleigh(neg_LdotV: f32) -> f32 {
    return FRAC_3_16_PI * (1 + (neg_LdotV * neg_LdotV));
}

// evaluates the henyey-greenstein phase function, which describes the likelihood
// of a mie scattering event scattering light from the light direction towards the view
fn henyey_greenstein(neg_LdotV: f32) -> f32 {
    let g = atmosphere.mie_asymmetry;
    let denom = 1.0 + g * g - 2.0 * g * neg_LdotV;
    return FRAC_4_PI * (1.0 - g * g) / (denom * sqrt(denom));
}

// ATMOSPHERE SAMPLING

struct AtmosphereSample {
    /// units: m^-1
    rayleigh_scattering: vec3<f32>,

    /// units: m^-1
    mie_scattering: f32,

    /// the sum of scattering and absorption. Since the phase function doesn't
    /// matter for this, we combine rayleigh and mie extinction to a single 
    //  value.
    //
    /// units: m^-1
    extinction: vec3<f32>
}

/// Samples atmosphere optical densities at a given radius
fn sample_atmosphere(r: f32) -> AtmosphereSample {
    let altitude = clamp(r, atmosphere.bottom_radius, atmosphere.top_radius) - atmosphere.bottom_radius;

    // atmosphere values at altitude
    let mie_density = exp(-atmosphere.mie_density_exp_scale * altitude);
    let rayleigh_density = exp(-atmosphere.rayleigh_density_exp_scale * altitude);
    var ozone_density: f32 = max(0.0, 1.0 - (abs(altitude - atmosphere.ozone_layer_altitude) / (atmosphere.ozone_layer_width * 0.5)));

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

/// evaluates L_scat, equation 3 in the paper, which gives the total single-order scattering towards the view at a single point
fn sample_local_inscattering(local_atmosphere: AtmosphereSample, ray_dir: vec3<f32>, world_pos: vec3<f32>) -> vec3<f32> {
    let local_r = length(world_pos);
    let local_up = normalize(world_pos);
    var inscattering = vec3(0.0);
    for (var light_i: u32 = 0u; light_i < lights.n_directional_lights; light_i++) {
        let light = &lights.directional_lights[light_i];

        let mu_light = dot((*light).direction_to_light, local_up);

        // -(L . V) == (L . -V). -V here is our ray direction, which points away from the view
        // instead of towards it (as is the convention for V)
        let neg_LdotV = dot((*light).direction_to_light, ray_dir);

        // Phase functions give the proportion of light
        // scattered towards the camera for each scattering type
        let rayleigh_phase = rayleigh(neg_LdotV);
        let mie_phase = henyey_greenstein(neg_LdotV);
        let scattering_coeff = local_atmosphere.rayleigh_scattering * rayleigh_phase + local_atmosphere.mie_scattering * mie_phase;

        let transmittance_to_light = sample_transmittance_lut(local_r, mu_light);
        let shadow_factor = transmittance_to_light * f32(!ray_intersects_ground(local_r, mu_light));

        // Transmittance from scattering event to light source
        let scattering_factor = shadow_factor * scattering_coeff;

        // Additive factor from the multiscattering LUT
        let psi_ms = sample_multiscattering_lut(local_r, mu_light);
        let multiscattering_factor = psi_ms * (local_atmosphere.rayleigh_scattering + local_atmosphere.mie_scattering);

        inscattering += (*light).color.rgb * (scattering_factor + multiscattering_factor);
    }
    return inscattering;
}

fn sample_sun_radiance(ray_dir_ws: vec3<f32>) -> vec3<f32> {
    let view_pos = get_view_position();
    let r = length(view_pos);
    let up = normalize(view_pos);
    let mu_view = dot(ray_dir_ws, up);
    let shadow_factor = f32(!ray_intersects_ground(r, mu_view));
    var sun_radiance = vec3(0.0);
    for (var light_i: u32 = 0u; light_i < lights.n_directional_lights; light_i++) {
        let light = &lights.directional_lights[light_i];
        let neg_LdotV = dot((*light).direction_to_light, ray_dir_ws);
        let angle_to_sun = fast_acos(clamp(neg_LdotV, -1.0, 1.0));
        let w = max(0.5 * fwidth(angle_to_sun), 1e-6);
        let sun_angular_size = (*light).sun_disk_angular_size;
        let sun_intensity = (*light).sun_disk_intensity;
        if sun_angular_size > 0.0 && sun_intensity > 0.0 {
            let factor = 1 - smoothstep(sun_angular_size * 0.5 - w, sun_angular_size * 0.5 + w, angle_to_sun);
            let sun_solid_angle = (sun_angular_size * sun_angular_size) * 0.25 * PI;
            sun_radiance += ((*light).color.rgb / sun_solid_angle) * sun_intensity * factor * shadow_factor;
        }
    }
    return sun_radiance;
}

// TRANSFORM UTILITIES

fn max_atmosphere_distance(r: f32, mu: f32) -> f32 {
    let t_top = distance_to_top_atmosphere_boundary(r, mu);
    let t_bottom = distance_to_bottom_atmosphere_boundary(r, mu);
    let hits = ray_intersects_ground(r, mu);
    return mix(t_top, t_bottom, f32(hits));
}

/// Returns the observer's position in the atmosphere
fn get_view_position() -> vec3<f32> {
    var world_pos = view.world_position * settings.scene_units_to_m + vec3(0.0, atmosphere.bottom_radius, 0.0);
    
    // If the camera is underground, clamp it to the ground surface along the local up.
    let r = length(world_pos);
    // Nudge r above ground to avoid sqrt cancellation, zero-length segments where 
    // r is equal to bottom_radius, which show up as black pixels
    let min_radius = atmosphere.bottom_radius + EPSILON;
    if r < min_radius {
        let up = normalize(world_pos);
        world_pos = up * min_radius;
    }

    return world_pos;
}

// We assume the `up` vector at the view position is the y axis, since the world is locally flat/level.
// t = distance along view ray in atmosphere space
// NOTE: this means that if your world is actually spherical, this will be wrong.
fn get_local_up(r: f32, t: f32, ray_dir: vec3<f32>) -> vec3<f32> {
    return normalize(vec3(0.0, r, 0.0) + t * ray_dir);
}

// Given a ray starting at radius r, with mu = cos(zenith angle),
// and a t = distance along the ray, gives the new radius at point t
fn get_local_r(r: f32, mu: f32, t: f32) -> f32 {
    return sqrt(t * t + 2.0 * r * mu * t + r * r);
}

// Convert uv [0.0 .. 1.0] coordinate to ndc space xy [-1.0 .. 1.0]
fn uv_to_ndc(uv: vec2<f32>) -> vec2<f32> {
    return uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0);
}

/// Convert ndc space xy coordinate [-1.0 .. 1.0] to uv [0.0 .. 1.0]
fn ndc_to_uv(ndc: vec2<f32>) -> vec2<f32> {
    return ndc * vec2(0.5, -0.5) + vec2(0.5);
}

/// Converts a direction in world space to atmosphere space
fn direction_world_to_atmosphere(dir_ws: vec3<f32>, up: vec3<f32>) -> vec3<f32> {
    // Camera forward in world space (-Z in view to world transform)
    let forward_ws = (view.world_from_view * vec4(0.0, 0.0, -1.0, 0.0)).xyz;
    let tangent_z = normalize(up * dot(forward_ws, up) - forward_ws);
    let tangent_x = cross(up, tangent_z);
    return vec3(
        dot(dir_ws, tangent_x),
        dot(dir_ws, up),
        dot(dir_ws, tangent_z),
    );
}

/// Converts a direction in atmosphere space to world space
fn direction_atmosphere_to_world(dir_as: vec3<f32>) -> vec3<f32> {
    let dir_ws = atmosphere_transforms.world_from_atmosphere * vec4(dir_as, 0.0);
    return dir_ws.xyz;
}

// Modified from skybox.wgsl. For this pass we don't need to apply a separate sky transform or consider camera viewport.
// Returns a normalized ray direction in world space.
fn uv_to_ray_direction(uv: vec2<f32>) -> vec3<f32> {
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

    let view_ray_direction = view_position_homogeneous.xyz / view_position_homogeneous.w;
    // Transforming the view space ray direction by the inverse view matrix, transforms the
    // direction to world space. Note that the w element is set to 0.0, as this is a
    // vector direction, not a position, That causes the matrix multiplication to ignore
    // the translations from the view matrix.
    let ray_direction = (view.world_from_view * vec4(view_ray_direction, 0.0)).xyz;

    return normalize(ray_direction);
}

fn zenith_azimuth_to_ray_dir(zenith: f32, azimuth: f32) -> vec3<f32> {
    let sin_zenith = sin(zenith);
    let mu = cos(zenith);
    let sin_azimuth = sin(azimuth);
    let cos_azimuth = cos(azimuth);
    return vec3(sin_azimuth * sin_zenith, mu, -cos_azimuth * sin_zenith);
}

struct RaymarchSegment {
    start: f32,
    end: f32,
}

fn get_raymarch_segment(r: f32, mu: f32) -> RaymarchSegment {
    // Get both intersection points with atmosphere
    let atmosphere_intersections = ray_sphere_intersect(r, mu, atmosphere.top_radius);
    let ground_intersections = ray_sphere_intersect(r, mu, atmosphere.bottom_radius);
    
    var segment: RaymarchSegment;
    
    if r < atmosphere.bottom_radius {
        // Inside planet - start from bottom of atmosphere
        segment.start = ground_intersections.y; // Use second intersection point with ground
        segment.end = atmosphere_intersections.y;
    } else if r < atmosphere.top_radius {
        // Inside atmosphere
        segment.start = 0.0;
        segment.end = select(atmosphere_intersections.y, ground_intersections.x, ray_intersects_ground(r, mu));
    } else {
        // Outside atmosphere
        if atmosphere_intersections.x < 0.0 {
            // No intersection with atmosphere
            return segment;
        }
        // Start at atmosphere entry, end at exit or ground
        segment.start = atmosphere_intersections.x;
        segment.end = select(atmosphere_intersections.y, ground_intersections.x, ray_intersects_ground(r, mu));
    }

    return segment;
}

struct RaymarchResult {
    inscattering: vec3<f32>,
    transmittance: vec3<f32>,
}

fn raymarch_atmosphere(
    pos: vec3<f32>,
    ray_dir: vec3<f32>,
    t_max: f32,
    max_samples: u32,
    uv: vec2<f32>,
    ground: bool
) -> RaymarchResult {
    let r = length(pos);
    let up = normalize(pos);
    let mu = dot(ray_dir, up);
    
    // Optimization: Reduce sample count at close proximity to the scene
    let sample_count = mix(1.0, f32(max_samples), saturate(t_max * 0.01));
    
    let segment = get_raymarch_segment(r, mu);
    let t_start = segment.start;
    var t_end = segment.end;
    
    t_end = min(t_end, t_max);
    let t_total = t_end - t_start;
    
    var result: RaymarchResult;
    result.inscattering = vec3(0.0);
    result.transmittance = vec3(1.0);
    
    // Skip if invalid segment
    if t_total <= 0.0 {
        return result;
    }
    
    var prev_t = t_start;
    var optical_depth = vec3(0.0);
    for (var s = 0.0; s < sample_count; s += 1.0) {
        // Linear distribution from atmosphere entry to exit/ground
        let t_i = t_start + t_total * (s + MIDPOINT_RATIO) / sample_count;
        let dt_i = (t_i - prev_t);
        prev_t = t_i;

        let sample_pos = pos + ray_dir * t_i;
        let local_r = length(sample_pos);
        let local_up = normalize(sample_pos);
        let local_atmosphere = sample_atmosphere(local_r);

        let sample_optical_depth = local_atmosphere.extinction * dt_i;
        optical_depth += sample_optical_depth;
        let sample_transmittance = exp(-sample_optical_depth);

        let inscattering = sample_local_inscattering(
            local_atmosphere,
            ray_dir,
            sample_pos
        );

        let s_int = (inscattering - inscattering * sample_transmittance) / local_atmosphere.extinction;
        result.inscattering += result.transmittance * s_int;

        result.transmittance *= sample_transmittance;
        if all(result.transmittance < vec3(0.001)) {
            break;
        }
    }

    // include reflected luminance from planet ground 
    if ground && ray_intersects_ground(r, mu) {
        for (var light_i: u32 = 0u; light_i < lights.n_directional_lights; light_i++) {
            let light = &lights.directional_lights[light_i];
            let light_dir = (*light).direction_to_light;
            let light_color = (*light).color.rgb;
            let transmittance_to_ground = exp(-optical_depth);
            // position on the sphere and get the sphere normal (up)
            let sphere_point = pos + ray_dir * t_end;
            let sphere_normal = normalize(sphere_point);
            let mu_light = dot(light_dir, sphere_normal);
            let transmittance_to_light = sample_transmittance_lut(0.0, mu_light);
            let light_luminance = transmittance_to_light * max(mu_light, 0.0) * light_color;
            // Normalized Lambert BRDF
            let ground_luminance = transmittance_to_ground * atmosphere.ground_albedo / PI;
            result.inscattering += ground_luminance * light_luminance;
        }
    }

    return result;
}
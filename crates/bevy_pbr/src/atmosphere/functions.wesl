//! NOTE FOR CONVENTIONS:
//! r:
//!   radius, or distance from planet center
//!
//! altitude:
//!   distance from planet **surface**
//!
//! mu:
//!   cosine of the zenith angle of a ray with
//!   respect to the planet normal
//!
//! atmosphere space:
//!   abbreviated as "as" (contrast with vs, cs, ws), this space is similar
//!   to view space, but with the camera positioned horizontally on the planet
//!   surface, so the horizon is a horizontal line centered vertically in the
//!   frame. This enables the non-linear latitude parametrization the paper uses
//!   to concentrate detail near the horizon

import bevy_render::maths::{PI, HALF_PI, PI_2, fast_acos, fast_acos_4, fast_atan2, ray_sphere_intersect};
import package::render::utils::interleaved_gradient_noise;

import package::atmosphere::{
    types::Atmosphere,
    bindings::{
        atmosphere, settings, view, lights, transmittance_lut, atmosphere_lut_sampler,
        multiscattering_lut, sky_view_lut, aerial_view_lut, atmosphere_transforms,
        medium_density_lut, medium_scattering_lut, medium_sampler,
        cloud_shadow_map, cloud_field_sdf,
    },
    bruneton_functions::{
        transmittance_lut_r_mu_to_uv, ray_intersects_ground,
        distance_to_top_atmosphere_boundary, distance_to_bottom_atmosphere_boundary
    },
    shadows::fetch_directional_shadow,
};

@if(CLOUDS_ENABLED)
import package::atmosphere::{
    bindings::stbn_texture,
    clouds::{
        get_cloud_coverage,
        cloud_field_uvw,
        get_cloud_medium_density,
        sample_cloud_field_density_and_grad,
        cloud_layer_segment, cloud_layer, get_cloud_medium_density_lod, get_cloud_medium_density_coarse,
        get_cloud_absorption_coeff,
        get_cloud_scattering_coeff,
        sample_cloud_contribution,
    },
};

// CONSTANTS
/// 1 / π
const FRAC_PI: f32 = 0.3183098862;
/// 1 / (2π)
const FRAC_2_PI: f32 = 0.15915494309;
/// 3 / (16π)
const FRAC_3_16_PI: f32 = 0.0596831036594607509;
/// 1 / (4π)
const FRAC_4_PI: f32 = 0.07957747154594767;
/// √2
const ROOT_2: f32 = 1.41421356;
/// 1 meter
const EPSILON: f32 = 1.0;
const MIN_EXTINCTION: vec3<f32> = vec3(1e-12);

/// Henyey-Greenstein phase function (normalized by 1/(4π)).
fn hg_phase(cos_theta: f32, g: f32) -> f32 {
    let gg = g * g;
    // p(θ) = (1/4π) * (1 - g²) / (1 + g² - 2g*cos(θ))^(3/2)
    return FRAC_4_PI * (1.0 - gg) / pow(max(1.0 + gg - 2.0 * g * cos_theta, 1e-6), 1.5);
}

/// Dual-lobe HG phase (Frostbite-style): blend a forward and backward lobe.
fn dual_lobe_hg_phase(cos_theta: f32, g_fwd: f32, g_bwd: f32, lobe_lerp: f32) -> f32 {
    return mix(hg_phase(cos_theta, g_fwd), hg_phase(cos_theta, g_bwd), lobe_lerp);
}

/// During raymarching, each segment is sampled at a single point. This constant determines
/// where in the segment that sample is taken (0.0 = start, 0.5 = middle, 1.0 = end).
/// We use 0.3 to sample closer to the start of each segment, which better approximates
/// the exponential falloff of atmospheric density.
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
    let v = saturate((r - atmosphere.inner_radius) / (atmosphere.outer_radius - atmosphere.inner_radius)); //TODO
    return unit_to_sub_uvs(vec2(u, v), vec2<f32>(settings.multiscattering_lut_size));
}

fn multiscattering_lut_uv_to_r_mu(uv: vec2<f32>) -> vec2<f32> {
    let adj_uv = sub_uvs_to_unit(uv, vec2<f32>(settings.multiscattering_lut_size));
    let r = mix(atmosphere.inner_radius, atmosphere.outer_radius, adj_uv.y);
    let mu = adj_uv.x * 2 - 1;
    return vec2(r, mu);
}

fn sky_view_lut_r_mu_azimuth_to_uv(r: f32, mu: f32, azimuth: f32) -> vec2<f32> {
    let u = (azimuth * FRAC_2_PI) + 0.5;

    let v_horizon = sqrt(r * r - atmosphere.inner_radius * atmosphere.inner_radius);
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
    let v_horizon = sqrt(r * r - atmosphere.inner_radius * atmosphere.inner_radius);
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
    let uv = transmittance_lut_r_mu_to_uv(atmosphere, r, mu);
    return textureSampleLevel(transmittance_lut, atmosphere_lut_sampler, uv, 0.0).rgb;
}

/// Reducible Mie/haze optical depth (∫ g·σ_s dt) from (r, mu) to the top of atmosphere.
/// Packed in the transmittance LUT alpha; used for similarity-scaled AP: T_sim = T · exp(τ_red).
fn sample_transmittance_lut_tau_red(r: f32, mu: f32) -> f32 {
    let uv = transmittance_lut_r_mu_to_uv(atmosphere, r, mu);
    return textureSampleLevel(transmittance_lut, atmosphere_lut_sampler, uv, 0.0).a;
}

fn sample_multiscattering_lut(r: f32, mu: f32) -> vec3<f32> {
    let uv = multiscattering_lut_r_mu_to_uv(r, mu);
    return textureSampleLevel(multiscattering_lut, atmosphere_lut_sampler, uv, 0.0).rgb;
}

fn sample_sky_view_lut(r: f32, ray_dir_as: vec3<f32>) -> vec3<f32> {
    let mu = ray_dir_as.y;
    let azimuth = fast_atan2(ray_dir_as.x, -ray_dir_as.z);
    let uv = sky_view_lut_r_mu_azimuth_to_uv(r, mu, azimuth);
    return textureSampleLevel(sky_view_lut, atmosphere_lut_sampler, uv, 0.0).rgb;
}

fn ndc_to_camera_dist(ndc: vec3<f32>) -> f32 {
    let view_pos = view.view_from_clip * vec4(ndc, 1.0);
    let p_view = view_pos.xyz / view_pos.w;
    let p_world = (view.world_from_view * vec4(p_view, 1.0)).xyz;
    let p_atmo = (atmosphere.world_to_atmosphere * vec4(p_world, 1.0)).xyz;
    let cam_atmo = (atmosphere.world_to_atmosphere * vec4(view.world_position, 1.0)).xyz;
    return length(p_atmo - cam_atmo);
}

/// RGB channels: total inscattered light along the camera ray to the current sample.
/// A channel: average transmittance across all wavelengths to the current sample.
fn sample_aerial_view_lut(uv: vec2<f32>, t: f32) -> vec3<f32> {
    let t_max = settings.aerial_view_lut_max_distance;
    let num_slices = f32(settings.aerial_view_lut_size.z);
    let num_samples = f32(settings.aerial_view_lut_samples);
    // LUT depth is squared in normalized slice parameter u
    let u = sqrt(saturate(t / t_max));
    // Each texel stores the value of the scattering integral over the whole slice,
    // which requires us to offset the w coordinate by half a slice. For
    // example, if we wanted the value of the integral at the boundary between slices,
    // we'd need to sample at the center of the previous slice, and vice-versa for
    // sampling in the center of a slice.
    let uvw = vec3(uv, saturate(u - 0.5 / num_slices));
    let sample = textureSampleLevel(aerial_view_lut, atmosphere_lut_sampler, uvw, 0.0);
    // Since sampling anywhere between w=0 and w=t_slice will clamp to the first slice,
    // we need to do a linear step over the first slice towards zero at the camera's
    // position to recover the correct integral value.
    let t_slice = t_max / num_slices;
    let u_end_first_slice =
        ((num_samples - 1.0 + MIDPOINT_RATIO) / num_samples) / num_slices;
    let t_first_slice = t_max * u_end_first_slice * u_end_first_slice;
    let fade = saturate(t / t_first_slice);
    // Recover the values from log space
    return exp(sample.rgb) * fade;
}

// ATMOSPHERE SAMPLING

const ABSORPTION_DENSITY: f32 = 0.0;
const SCATTERING_DENSITY: f32 = 1.0;

/// samples from the atmosphere density LUT.
///
/// calling with `component = 0.0` will return the atmosphere's absorption density,
/// while calling with `component = 1.0` will return the atmosphere's scattering density.
fn sample_density_lut(r: f32, component: f32) -> vec3<f32> {
    // sampler clamps to [0, 1] anyways, no need to clamp the altitude
    let normalized_altitude = (r - atmosphere.inner_radius) / (atmosphere.outer_radius - atmosphere.inner_radius);
    let uv = vec2(1.0 - normalized_altitude, component);
    return textureSampleLevel(medium_density_lut, medium_sampler, uv, 0.0).xyz;
}


const PHASE_MAPPING_N: f32 = 0.5;

/// samples from the atmosphere scattering LUT. `neg_LdotV` is the dot product
/// of the light direction and the incoming view vector.
/// Nonlinear phase mapping to mitigate banding in low-resolution LUTs.
fn sample_scattering_lut(r: f32, neg_LdotV: f32) -> vec3<f32> {
    let normalized_altitude = (r - atmosphere.inner_radius) / (atmosphere.outer_radius - atmosphere.inner_radius);
    let phase_uv = 0.5 + 0.5 * sign(neg_LdotV) * (1.0 - pow(1.0 - abs(neg_LdotV), PHASE_MAPPING_N));
    let uv = vec2(1.0 - normalized_altitude, phase_uv);
    return textureSampleLevel(medium_scattering_lut, medium_sampler, uv, 0.0).xyz;
}

@if(CLOUDS_ENABLED)
struct LightBasis {
    x: vec3<f32>,
    y: vec3<f32>,
};

@if(CLOUDS_ENABLED)
fn safe_normalize(v: vec3<f32>) -> vec3<f32> {
    let l2 = dot(v, v);
    if (l2 <= 1e-12) {
        return vec3(0.0, 1.0, 0.0);
    }
    return v * inverseSqrt(l2);
}

@if(CLOUDS_ENABLED)
fn build_light_basis(light_dir: vec3<f32>) -> LightBasis {
    // Pick a vector not parallel to `light_dir`, then build an orthonormal basis.
    // IMPORTANT: keep this selection stable; switching too early causes the basis to “flip”
    // when the light gets moderately close to zenith, which looks like the shadow map drifts/rotates.
    let world_up = vec3(0.0, 1.0, 0.0);
    let a = select(world_up, vec3(1.0, 0.0, 0.0), abs(dot(light_dir, world_up)) > 0.999);
    let x = safe_normalize(cross(a, light_dir));
    let y = cross(light_dir, x);
    return LightBasis(x, y);
}

/// Helper for cloud shadow PCF: evaluates transmittance from shadow map data.
@if(CLOUDS_ENABLED)
fn cloud_shadow_transmittance(data: vec3<f32>, d_sample: f32) -> f32 {
    let front_depth = data.r * 1000.0;
    let mean_ext = data.g;
    let max_optical_depth = data.b;
    let delta = max(0.0, d_sample - front_depth);
    var tau = mean_ext * delta;
    tau = min(tau, max_optical_depth);
    return exp(-tau);
}

/// Ground illuminance variant: uses only the stored optical depth (B channel), ignoring front depth.
@if(CLOUDS_ENABLED)
fn cloud_shadow_optical_depth(data: vec3<f32>) -> f32 {
    let optical_depth = data.b;
    return exp(-optical_depth);
}

@if(CLOUDS_ENABLED)
fn snap_anchor_to_texel_grid(anchor: vec3<f32>, basis: LightBasis, extent: f32, size: vec2<u32>) -> vec3<f32> {
    // Keep producer/consumer aligned by snapping the anchor to the shadow-map texel grid.
    let res = vec2<f32>(size);
    let texel_size = (2.0 * extent) / max(res, vec2(1.0));

    let ax = dot(anchor, basis.x);
    let ay = dot(anchor, basis.y);

    let snapped_ax = round(ax / texel_size.x) * texel_size.x;
    let snapped_ay = round(ay / texel_size.y) * texel_size.y;

    let dx = snapped_ax - ax;
    let dy = snapped_ay - ay;
    return anchor + basis.x * dx + basis.y * dy;
}

/// Unreal-style cloud shadow map evaluation.
///
/// The `cloud_shadow_map` stores:
/// - R: front depth (meters) from the light-volume near plane
/// - G: mean extinction (1/m)
/// - B: max optical depth (unitless)
///
/// For a world-space sample point, we compute:
///   tau = mean_ext * (d_sample - d_front), clamped by max_optical_depth
///   T = exp(-tau)
///
/// When `use_optical_depth_only` is true (e.g. for ground illuminance), front depth is ignored
/// and transmittance is computed from the stored optical depth only: T = exp(-optical_depth).
@if(CLOUDS_ENABLED)
/// Cloud transmittance toward the light for points outside the shadow-map volume (far,
/// grazing-sun lookups). Returning 1 there lit the air under the cloud deck near the horizon
/// (3-4x too bright at 3 deg sun). Short stratified density march along the light.
/// 2 samples (4 -> 2: clouds_raymarch -18..-25 %, total -1.4 ms hero:35). render_sky (ground,
/// far haze) marches the mip-3 noise density: the coarse envelope left distant ground ~27 % too
/// bright vs CLOUD_REFERENCE (hero:35), the fine density matches (-2 %). The cloud pass keeps the
/// cheaper coarse envelope at half strength (fine there over-brightened up:35 by up to +36 %).
const CLOUD_FAR_SAMPLES: u32 = 2u;
const CLOUD_FAR_MAX_M: f32 = 80000.0;
const CLOUD_FAR_TAP_M: f32 = 1500.0;
const CLOUD_FAR_LONG_SAMPLES: u32 = 16u;
const CLOUD_FAR_GRAZING_MU: f32 = 0.2; // sun below ~11.5 deg
const CLOUD_FAR_COARSE_SCALE: f32 = 0.5;
@if(CLOUDS_ENABLED)
fn cloud_far_transmittance(world_pos: vec3<f32>, direction_to_light: vec3<f32>, use_optical_depth_only: bool) -> f32 {
    var od = 0.0;
    @if(CLOUDS_ENABLED) {
    let seg = cloud_layer_segment(world_pos, direction_to_light);
    if (seg.z > 0.5) {
        let t0 = max(seg.x, 0.0);
        // Grazing sun: long paths through the layer need more taps, or distant cloud saw no
        // occluder and lit up (sun:3 horizon specks).
        let full = seg.y - t0;
        var far_max = CLOUD_FAR_MAX_M;
        var n_far = CLOUD_FAR_SAMPLES;
        var far_mip = 3.0;
        // ~1.5 km tap spacing (2 taps over a 10+ km grazing path skipped whole clouds).
        if (dot(direction_to_light, normalize(world_pos)) < CLOUD_FAR_GRAZING_MU) {
            n_far = u32(clamp(ceil(min(full, CLOUD_FAR_MAX_M) / CLOUD_FAR_TAP_M), f32(CLOUD_FAR_SAMPLES), f32(CLOUD_FAR_LONG_SAMPLES)));
        }
        let t1 = min(seg.y, t0 + far_max);
        let dt = (t1 - t0) / f32(n_far);
        let sigma_t = get_cloud_scattering_coeff() + get_cloud_absorption_coeff();
        for (var i = 0u; i < n_far; i++) {
            let q = world_pos + direction_to_light * (t0 + (f32(i) + 0.5) * dt);
            var dq = get_cloud_medium_density_lod(length(q), q, far_mip);
            @if(!CLOUD_PASS) { dq = get_cloud_medium_density_coarse(length(q), q) * CLOUD_FAR_COARSE_SCALE; }
            od += dq * sigma_t * dt;
        }
    }
    }
    od *= settings.cloud_shadow_map_strength;
    if (use_optical_depth_only) {
        return exp(-od);
    }
    return exp(-od);
}

@if(CLOUDS_ENABLED)
fn sample_cloud_shadow_map(world_pos: vec3<f32>, direction_to_light: vec3<f32>, use_optical_depth_only: bool) -> f32 {

    let extent = settings.cloud_shadow_map_extent;
    // Keep consumer consistent with producer: half-depth is derived from extent (Unreal-style).
    let half_depth = extent * 2.0;
    if (extent <= 0.0 || half_depth <= 0.0) {
        return 1.0;
    }

    // IMPORTANT: use the same trace direction as the compute pass:
    // light -> surface (opposite of Bevy's `direction_to_light` which is surface -> light).
    let trace_dir = safe_normalize(-direction_to_light);
    let basis = build_light_basis(trace_dir);

    // Anchor must match the compute shader's anchor to keep lookups stable.
    var anchor = get_view_position();
    anchor = snap_anchor_to_texel_grid(anchor, basis, extent, settings.cloud_shadow_map_size);
    let rel = world_pos - anchor;

    // Map world point to the orthographic light volume.
    let x = dot(rel, basis.x);
    let y = dot(rel, basis.y);
    if (abs(x) > extent || abs(y) > extent) {
        return cloud_far_transmittance(world_pos, direction_to_light, use_optical_depth_only);
    }

    // Depth from near plane along light direction.
    // Near plane is located at `anchor - light_dir * half_depth`.
    let d_sample = dot(rel, trace_dir) + half_depth;
    let max_t = 2.0 * half_depth;
    if (d_sample <= 0.0 || d_sample >= max_t) {
        return cloud_far_transmittance(world_pos, direction_to_light, use_optical_depth_only);
    }

    let uv = vec2(x, y) / (2.0 * extent) + vec2(0.5);

    @if(CLOUD_SHADOW_SIMPLE_SAMPLING) {
    let d = textureSampleLevel(cloud_shadow_map, atmosphere_lut_sampler, uv, 0.0).rgb;
    if (use_optical_depth_only) {
        return cloud_shadow_optical_depth(d);
    }
    return cloud_shadow_transmittance(d, d_sample);
    } @else {
    // 4-tap PCF (2x2 half-texel) to reduce aliasing and soften shadow edges.
    // Samples are offset by ±0.25 texels so the 2x2 footprint covers the sampling area.
    let size = vec2<f32>(settings.cloud_shadow_map_size);
    let texel = vec2(1.0 / max(size.x, 1.0), 1.0 / max(size.y, 1.0));
    let o = 0.25 * texel;

    let d00 = textureSampleLevel(cloud_shadow_map, atmosphere_lut_sampler, uv + vec2(-o.x, -o.y), 0.0).rgb;
    let d10 = textureSampleLevel(cloud_shadow_map, atmosphere_lut_sampler, uv + vec2(o.x, -o.y), 0.0).rgb;
    let d01 = textureSampleLevel(cloud_shadow_map, atmosphere_lut_sampler, uv + vec2(-o.x, o.y), 0.0).rgb;
    let d11 = textureSampleLevel(cloud_shadow_map, atmosphere_lut_sampler, uv + vec2(o.x, o.y), 0.0).rgb;

    let t00 = select(cloud_shadow_transmittance(d00, d_sample), cloud_shadow_optical_depth(d00), use_optical_depth_only);
    let t10 = select(cloud_shadow_transmittance(d10, d_sample), cloud_shadow_optical_depth(d10), use_optical_depth_only);
    let t01 = select(cloud_shadow_transmittance(d01, d_sample), cloud_shadow_optical_depth(d01), use_optical_depth_only);
    let t11 = select(cloud_shadow_transmittance(d11, d_sample), cloud_shadow_optical_depth(d11), use_optical_depth_only);

    return (t00 + t10 + t01 + t11) * 0.25;
    }
}

/// evaluates L_scat, equation 3 in the paper, which gives the total single-order scattering towards the view at a single point
fn sample_local_inscattering(local_scattering: vec3<f32>, ray_dir: vec3<f32>, world_pos: vec3<f32>, pixel_coords: vec2<f32>) -> vec3<f32> {
    let local_r = length(world_pos);
    let local_up = normalize(world_pos);
    var inscattering = vec3(0.0);
    
    
    for (var light_i: u32 = 0u; light_i < lights.n_directional_lights; light_i++) {
        let light = &lights.directional_lights[light_i];

        let mu_light = dot((*light).direction_to_light, local_up);

        // NOTE ON SIGN CONVENTIONS:
        // `ray_dir` points *away* from the camera (camera -> sample).
        // `direction_to_light` (L) points from the sample *towards the light* (sample -> sun).
        //
        // Many texts define phase in terms of propagation directions ωi (sun -> sample) and ωo (sample -> camera),
        // in which case: ωi = -L and ωo = -ray_dir, so:
        //   cos(theta) = dot(ωi, ωo) = dot(-L, -ray_dir) = dot(L, ray_dir)
        //
        // So for our phase functions, the correct cosine is simply dot(L, ray_dir).
        let neg_LdotV = dot((*light).direction_to_light, ray_dir);

        let transmittance_to_light = sample_transmittance_lut(local_r, mu_light);
        var shadow_factor = transmittance_to_light * f32(!ray_intersects_ground(local_r, mu_light));
        var scattering_coeff = sample_scattering_lut(local_r, neg_LdotV);
        
        var cloud_sun_t = 1.0;
        @if(CLOUDS_ENABLED && !ATM_DBG_NOSHADOW) {
        // Compute volumetric shadow from clouds via the cloud shadow map (stable at grazing angles).
        cloud_sun_t = sample_cloud_shadow_map(world_pos, (*light).direction_to_light, false);
        shadow_factor *= cloud_sun_t;
        }

        // With CLOUD_PASS (render_sky), clouds are integrated by the dedicated low-res
        // `clouds_raymarch` pass and composited afterwards (the sky-view/aerial LUTs and the cloud
        // pass must not add them again); only the env probe still
        // integrates cloud media here.
        @if(CLOUDS_ENABLED && ATMOSPHERE_ENV_PROBE) {
        // Sample cloud *medium density* at this point (includes `cloud_layer.cloud_density`)
        // so clouds respond consistently across scattering vs shadowing.
        let cloud_density = get_cloud_medium_density(local_r, world_pos);
        // NUBIS: Add volumetric cloud scattering with proper physical integration
        // Clouds contribute to inscattering via Henyey-Greenstein phase function

        // Cloud scattering coefficient: σ_s_cloud = density * scattering_coeff
        // Using physically correct coefficients (units: m^-1 per unit density)
        // Density is normalized [0, 1], coefficients represent actual physical values
        // Water droplet clouds have scattering ~0.0008-0.001 m^-1 per unit density
        let cloud_scattering_coeff = cloud_density * get_cloud_scattering_coeff();
        
        // Henyey-Greenstein phase function for anisotropic cloud scattering
        // NUBIS/Frostbite-style: dual-lobe HG to better match real clouds.
        //
        // `neg_LdotV` here is dot(L, ray_dir); see sign convention note above.
        let cos_theta = clamp(neg_LdotV, -1.0, 1.0);

        // Forward lobe (strong forward scattering) and a weaker backward lobe.
        // Matches the approach used in `bevy-volumetric-clouds` (Frostbite-inspired).
        let g_fwd = 0.85;
        let g_bwd = -0.2;
        let lobe_lerp = 0.5;
        let cloud_phase = dual_lobe_hg_phase(cos_theta, g_fwd, g_bwd, lobe_lerp);
        
        // Add cloud scattering contribution: σ_s_cloud * p(θ)
        scattering_coeff += cloud_scattering_coeff * cloud_phase;
        }

        // Sample directional light shadow map for volumetric shadow shafts.
        // Use zero normal bias: unlike mesh shading there is no surface normal, and using
        // `-ray_dir` (toward the camera) pushes samples along the view ray away from nearby
        // occluders, which causes light leaks next to buildings and other low geometry.
        var shadow_map_factor: f32 = 1.0;
        // Pipelines without shadow map bindings (e.g. the clouds environment probe) set
        // ATMOSPHERE_NO_SHADOW_MAPS to compile the shadow map lookup out.
        @if(!ATMOSPHERE_NO_SHADOW_MAPS && !ATM_DBG_NOSHADOW) {
            if (settings.shadows_enabled != 0u) {
                shadow_map_factor = fetch_directional_shadow(light_i, vec4(world_pos, 1.0), vec3(0.0));
            }
        }

        // Transmittance from scattering event to light source
        let scattering_factor = shadow_factor * shadow_map_factor * scattering_coeff;

        // Additive factor from the multiscattering LUT
        let psi_ms = sample_multiscattering_lut(local_r, mu_light);
        var multiscattering_factor = psi_ms * local_scattering;
        @if(ATM_DBG_NOMS) { multiscattering_factor = vec3(0.0); }

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

fn calculate_visible_sun_ratio(atmosphere: Atmosphere, r: f32, mu: f32, sun_angular_size: f32) -> f32 {
    let inner_radius = atmosphere.inner_radius;
    // Calculate the angle between horizon and sun center
    // Invert the horizon angle calculation to fix shading direction
    let horizon_cos = -sqrt(1.0 - (inner_radius * inner_radius) / (r * r));
    let horizon_angle = fast_acos_4(horizon_cos);
    let sun_zenith_angle = fast_acos_4(mu);

    // If sun is completely above horizon
    if sun_zenith_angle + sun_angular_size * 0.5 <= horizon_angle {
        return 1.0;
    }

    // If sun is completely below horizon
    if sun_zenith_angle - sun_angular_size * 0.5 >= horizon_angle {
        return 0.0;
    }

    // Calculate partial visibility using circular segment area formula
    let d = (horizon_angle - sun_zenith_angle) / (sun_angular_size * 0.5);
    let visible_ratio = 0.5 + d * 0.5;
    return clamp(visible_ratio, 0.0, 1.0);
}

// TRANSFORM UTILITIES

/// Clamp a position to the planet surface (with a small epsilon) to avoid underground artifacts.
fn clamp_to_surface(atmosphere: Atmosphere, position: vec3<f32>) -> vec3<f32> {
    let min_radius = atmosphere.inner_radius + EPSILON;
    let r = length(position);
    if r < min_radius {
        let up = normalize(position);
        return up * min_radius;
    }
    return position;
}

fn max_atmosphere_distance(r: f32, mu: f32) -> f32 {
    let t_top = distance_to_top_atmosphere_boundary(atmosphere, r, mu);
    let t_bottom = distance_to_bottom_atmosphere_boundary(r, mu);
    let hits = ray_intersects_ground(r, mu);
    return mix(t_top, t_bottom, f32(hits));
}

/// Returns the observer's position in the atmosphere
fn get_view_position() -> vec3<f32> {
    let atmo_pos = (atmosphere.world_to_atmosphere * vec4(view.world_position, 1.0)).xyz;
    return clamp_to_surface(atmosphere, atmo_pos);
}

/// We assume the `up` vector at the view position is the y axis, since the world is locally flat/level.
/// t = distance along view ray in atmosphere space
/// NOTE: this means that if your world is actually spherical, this will be wrong.
fn get_local_up(r: f32, t: f32, ray_dir: vec3<f32>) -> vec3<f32> {
    return normalize(vec3(0.0, r, 0.0) + t * ray_dir);
}

/// Given a ray starting at radius r, with mu = cos(zenith angle),
/// and a t = distance along the ray, gives the new radius at point t
fn get_local_r(r: f32, mu: f32, t: f32) -> f32 {
    return sqrt(t * t + 2.0 * r * mu * t + r * r);
}

/// Convert uv [0.0 .. 1.0] coordinate to ndc space xy [-1.0 .. 1.0]
fn uv_to_ndc(uv: vec2<f32>) -> vec2<f32> {
    return uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0);
}

/// Convert ndc space xy coordinate [-1.0 .. 1.0] to uv [0.0 .. 1.0]
fn ndc_to_uv(ndc: vec2<f32>) -> vec2<f32> {
    return ndc * vec2(0.5, -0.5) + vec2(0.5);
}

/// Converts a direction in world space to atmosphere space
fn direction_world_to_atmosphere(dir_ws: vec3<f32>) -> vec3<f32> {
    let dir_as = atmosphere_transforms.atmosphere_from_world * vec4(dir_ws, 0.0);
    return dir_as.xyz;
}

/// Converts a direction in atmosphere space to world space
fn direction_atmosphere_to_world(dir_as: vec3<f32>) -> vec3<f32> {
    let dir_ws = atmosphere_transforms.world_from_atmosphere * vec4(dir_as, 0.0);
    return dir_ws.xyz;
}

// Modified from skybox.wesl. For this pass we don't need to apply a separate sky transform or consider camera viewport.
/// Returns a normalized ray direction in world space.
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
    // Transforming the view space ray direction by the inverse view matrix transforms the
    // direction to world space. Note that the w element is set to 0.0, as this is a
    // vector direction, not a position, that causes the matrix multiplication to ignore
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

// Inverse CDF mapping for cloud-aware global stratification.
// Maps u in [0,1] to a distance t along the ray in [t_start, t_end],
// given piecewise weights for pre/cloud/post segments.
fn cloud_shadow_inv_cdf(
    u: f32,
    t_start: f32,
    cloud_start: f32,
    cloud_end: f32,
    t_end: f32,
    w_pre: f32,
    w_cloud: f32,
    w_post: f32,
    w_sum: f32,
) -> f32 {
    let x = u * w_sum;
    if (x < w_pre) {
        return t_start + (select(0.0, x / w_pre, w_pre > 0.0)) * (cloud_start - t_start);
    }
    let x2 = x - w_pre;
    if (x2 < w_cloud) {
        return cloud_start + (select(0.0, x2 / w_cloud, w_cloud > 0.0)) * (cloud_end - cloud_start);
    }
    let x3 = x2 - w_cloud;
    return cloud_end + (select(0.0, x3 / max(1e-6, w_post), w_post > 0.0)) * (t_end - cloud_end);
}

fn get_raymarch_segment(r: f32, mu: f32) -> RaymarchSegment {
    // Get both intersection points with atmosphere
    let atmosphere_intersections = ray_sphere_intersect(r, mu, atmosphere.outer_radius);
    let ground_intersections = ray_sphere_intersect(r, mu, atmosphere.inner_radius);

    var segment: RaymarchSegment;

    if r < atmosphere.inner_radius {
        // Inside planet - start from bottom of atmosphere
        segment.start = ground_intersections.y; // Use second intersection point with ground
        segment.end = atmosphere_intersections.y;
    } else if r < atmosphere.outer_radius {
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

    // Convert UV to pixel coordinates for noise jittering
    // Assuming viewport size from view uniform (typically available)
    let pixel_coords = uv * view.viewport.zw;
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

    var optical_depth = vec3(0.0);

    // Convert the sample count into a hard budget.
    let sample_budget = max(1u, u32(sample_count));

    @if(CLOUDS_ENABLED && ATMOSPHERE_ENV_PROBE) {
    // Cloud-aware global stratification (no temporal noise):
    // Instead of using separate per-segment grids (which can introduce visible boundaries),
    // we stratify over [0,1] globally and map through a piecewise-linear CDF that allocates
    // more samples to the cloud layer region.

    let seg = cloud_layer_segment(pos, ray_dir);
    // IMPORTANT:
    // If the view ray does *not* intersect the cloud shell, we must still integrate the atmosphere.
    // A previous version of this logic set all segment lengths to 0 when `has_cloud_layer == false`,
    // which collapses the inverse CDF to a constant and yields dt==0 => no scattering (black sky).
    let cloud_start_raw = clamp(seg.x, t_start, t_end);
    let cloud_end_raw = clamp(seg.y, t_start, t_end);
    let has_cloud_layer = (seg.z > 0.5) && (cloud_end_raw > cloud_start_raw);

    // When there is no cloud segment, treat the entire raymarch segment as "outside".
    let cloud_start = select(t_end, cloud_start_raw, has_cloud_layer);
    let cloud_end = select(t_end, cloud_end_raw, has_cloud_layer);

    let len_pre = max(0.0, cloud_start - t_start);
    let len_cloud = max(0.0, cloud_end - cloud_start);
    let len_post = max(0.0, t_end - cloud_end);

    // Importance multiplier for the cloud layer segment.
    // Higher => more samples in clouds, fewer outside, with *continuous* sampling across boundaries.
    let cloud_importance = 16.0;

    var w_pre = len_pre;
    var w_cloud = len_cloud * cloud_importance;
    var w_post = len_post;

    let w_sum = max(1e-6, w_pre + w_cloud + w_post);

    // Gradient scale factor (unit: meters) to turn |∇density| (1/m) into a dimensionless importance.
    const GRAD_IMPORTANCE_M: f32 = 1500.0;
    const CLOUD_MAX_SUBSTEPS: u32 = 2u;

    // STBN: use different layer per frame for temporal stratification
    let stbn_dims = textureDimensions(stbn_texture);
    let stbn_use = all(stbn_dims > vec2(1u));
    let stbn_layer = select(0, i32(view.frame_count % u32(textureNumLayers(stbn_texture))), stbn_use);
    let stbn_px = vec2<i32>(floor(pixel_coords)) % vec2<i32>(stbn_dims);

    var stop: bool = false;
    for (var s: u32 = 0u; s < sample_budget; s += 1u) {
        if (stop) { break; }

        let u0 = f32(s) / f32(sample_budget);
        let u1 = f32(s + 1u) / f32(sample_budget);
        var j: f32;
        if (stbn_use) {
            let stbn_noise = textureLoad(stbn_texture, stbn_px, stbn_layer, 0);
            j = fract(stbn_noise.r + f32(s) * 0.618033988749895);
        } else {
            j = interleaved_gradient_noise(pixel_coords, 500u + s);
        }
        let u = mix(u0, u1, j);

        let t0 = cloud_shadow_inv_cdf(u0, t_start, cloud_start, cloud_end, t_end, w_pre, w_cloud, w_post, w_sum);
        let t1 = cloud_shadow_inv_cdf(u1, t_start, cloud_start, cloud_end, t_end, w_pre, w_cloud, w_post, w_sum);
        let dt = max(0.0, t1 - t0);
        if (dt <= 0.0) { continue; }

        let t = cloud_shadow_inv_cdf(u, t_start, cloud_start, cloud_end, t_end, w_pre, w_cloud, w_post, w_sum);
        let sample_pos = pos + ray_dir * t;
        let local_r = length(sample_pos);

        // Inside-cloud substepping for sharp density transitions.
        let in_cloud = has_cloud_layer && (t >= cloud_start) && (t < cloud_end);
        var sub_steps: u32 = 1u;
        if (in_cloud) {
            let field = sample_cloud_field_density_and_grad(local_r, sample_pos);
            let g = max(0.0, field.y * GRAD_IMPORTANCE_M);
            sub_steps = min(CLOUD_MAX_SUBSTEPS, max(1u, u32(ceil(1.0 + g))));
        }

        let sub_dt = dt / f32(sub_steps);
        for (var k: u32 = 0u; k < sub_steps; k += 1u) {
            if (stop) { break; }
            // Jitter within each sub-step using STBN for temporal stratification.
            var j2: f32;
            if (stbn_use) {
                let stbn_noise2 = textureLoad(stbn_texture, stbn_px, stbn_layer, 0);
                j2 = fract(stbn_noise2.g + f32(s * 8u + k) * 0.618033988749895);
            } else {
                j2 = interleaved_gradient_noise(pixel_coords, 2000u + s * 8u + k);
            }
            let sub_t = t0 + (f32(k) + clamp(j2, 0.05, 0.95)) * sub_dt;
            let p = pos + ray_dir * sub_t;
            let r_p = length(p);

            let absorption = sample_density_lut(r_p, ABSORPTION_DENSITY);
            let scattering = sample_density_lut(r_p, SCATTERING_DENSITY);
            var extinction = absorption + scattering;

            // Cloud extinction on the view ray.
            let cloud_density = get_cloud_medium_density(r_p, p);
            if (cloud_density > 0.0) {
                let cloud_extinction = cloud_density * (get_cloud_scattering_coeff() + get_cloud_absorption_coeff());
                extinction += vec3(cloud_extinction);
            }

            let sample_optical_depth = extinction * sub_dt;
            optical_depth += sample_optical_depth;
            let sample_transmittance = exp(-sample_optical_depth);

            let inscattering = sample_local_inscattering(scattering, ray_dir, p, pixel_coords);
            let s_int = (inscattering - inscattering * sample_transmittance) / max(extinction, MIN_EXTINCTION);
            result.inscattering += result.transmittance * s_int;
            result.transmittance *= sample_transmittance;

            if all(result.transmittance < vec3(0.001)) {
                stop = true;
            }
        }
    }
    } @else {
    // Quadratically spaced segments with a jittered sample inside each one. Uniform steps
    // (thousands of meters long on sky rays) skipped the dense near-surface Mie/haze layers and
    // dropped the last segment, leaving the sky 2-3x too dark and blue-shifted.
    let n_steps = max(1u, u32(sample_count));
    for (var s = 0u; s < n_steps; s += 1u) {
        let jitter = interleaved_gradient_noise(pixel_coords, s); // [0, 1]
        let u0 = f32(s) / f32(n_steps);
        let u1 = f32(s + 1u) / f32(n_steps);
        let t0 = t_start + t_total * u0 * u0;
        let t1 = t_start + t_total * u1 * u1;
        let t_i = mix(t0, t1, jitter);
        let dt_i = t1 - t0;

        let sample_pos = pos + ray_dir * t_i;
        let local_r = length(sample_pos);

        let absorption = sample_density_lut(local_r, ABSORPTION_DENSITY);
        let scattering = sample_density_lut(local_r, SCATTERING_DENSITY);
        let extinction = absorption + scattering;

        let sample_optical_depth = extinction * dt_i;
        optical_depth += sample_optical_depth;
        let sample_transmittance = exp(-sample_optical_depth);

        let inscattering = sample_local_inscattering(scattering, ray_dir, sample_pos, pixel_coords);
        let s_int = (inscattering - inscattering * sample_transmittance) / max(extinction, MIN_EXTINCTION);
        result.inscattering += result.transmittance * s_int;
        result.transmittance *= sample_transmittance;
        if all(result.transmittance < vec3(0.001)) {
            break;
        }
    }
    }

    // include reflected luminance from planet ground
    if ground && ray_intersects_ground(r, mu) {
        // position on the sphere and get the sphere normal (up)
        let sphere_point = pos + ray_dir * t_end;
        let sphere_normal = normalize(sphere_point);
        for (var light_i: u32 = 0u; light_i < lights.n_directional_lights; light_i++) {
            let light = &lights.directional_lights[light_i];
            let light_dir = (*light).direction_to_light;
            let light_color = (*light).color.rgb;
            let transmittance_to_ground = exp(-optical_depth);
            let mu_light = dot(light_dir, sphere_normal);
            let transmittance_to_light = sample_transmittance_lut(atmosphere.inner_radius, mu_light);
            var light_luminance = transmittance_to_light * max(mu_light, 0.0) * light_color;
            @if(CLOUDS_ENABLED) {
            light_luminance *= sample_cloud_shadow_map(sphere_point, light_dir, true);
            }
            // Normalized Lambert BRDF
            let ground_luminance = transmittance_to_ground * atmosphere.ground_albedo / PI;
            result.inscattering += ground_luminance * light_luminance;
        }
        // Skylight on the ground (Lambert): dominant at low sun and missing before
        // (ground was 3-50x too dark in blue at 3 deg sun elevation).
        var e_sky = ground_sky_irradiance(sphere_point, pixel_coords);
        // (Not in the sky-view LUT pass itself: it cannot read the LUT it writes.)
        @if(CLOUDS_ENABLED && !SKY_VIEW_LUT_PASS) {
            // The sky-view LUT has no clouds: per skylight direction, replace the clear sky by
            // dimmer cloud bases using the cloud transmittance along it (floor = light the clouds
            // scatter down).
            e_sky = ground_sky_irradiance_clouds(sphere_point);
        }
        result.inscattering += exp(-optical_depth) * atmosphere.ground_albedo / PI * e_sky;
    }

    return result;
}

/// Sky irradiance on a horizontal ground point: cosine-weighted directions (E = pi * mean(L)),
/// each a short quadratically spaced single + LUT-multiple scattering march.
@if(CLOUDS_ENABLED)
fn ground_sky_irradiance_clouds(p: vec3<f32>) -> vec3<f32> {
    let n = normalize(p);
    let tx = normalize(select(vec3(1.0, 0.0, 0.0), vec3(0.0, 0.0, 1.0), abs(n.x) > 0.9) - n * dot(n, select(vec3(1.0, 0.0, 0.0), vec3(0.0, 0.0, 1.0), abs(n.x) > 0.9)));
    let ty = cross(n, tx);
    let r_cam = length(get_view_position());
    // Sunlight diffusely transmitted through a cloud of optical thickness tau (two-stream,
    // g = 0.85): T_diff ~ 1 / (1 + 0.75 (1 - g) tau), seen from below as Lambertian radiance.
    var e_top = vec3(0.0);
    if (lights.n_directional_lights > 0u) {
        let l = lights.directional_lights[0].direction_to_light;
        let mu_l = dot(l, n);
        e_top = lights.directional_lights[0].color.rgb * max(mu_l, 0.0)
            * sample_transmittance_lut(cloud_layer.cloud_layer_end, mu_l) / PI;
        // Diffuse transmission depends on the sun's incidence at the cloud top: Monte Carlo slab
        // (HG 0.85, conservative, validation/sky/slab_t.py) fits T ~ (0.35 + 0.9 mu0) / (1 + 0.1125 tau)
        // within ~8 % for tau >= 5; at grazing sun (mu0 0.05) that is 0.4x the mu0-free form.
        e_top *= 0.35 + 0.9 * max(mu_l, 0.0);
    }
    // Clear-sky irradiance from the sky-view LUT (cosine-weighted directions).
    var sky = vec3(0.0);
    for (var k = 0u; k < GROUND_SKY_DIRS; k++) {
        let u = (f32(k) + 0.5) / f32(GROUND_SKY_DIRS);
        let phi = f32(k) * 2.399963;
        let d = normalize(n * sqrt(1.0 - u) + (tx * cos(phi) + ty * sin(phi)) * sqrt(u));
        sky += sample_sky_view_lut(r_cam, direction_world_to_atmosphere(d));
    }
    sky *= PI / f32(GROUND_SKY_DIRS);
    // Cloud AO (UE5 volumetric clouds): sky visibility from the blurred column optical depth of
    // the cloud field above the point, exp(-tau). Outside the field, a vertical density march.
    // Hemispherical (diffuse) transmittance of a slab of vertical optical depth tau: the
    // diffusivity approximation T_diff ~ exp(-1.66 tau) (Elsasser 1942; standard two-stream
    // radiative transfer), since skylight arrives along slant paths.
    let tau = cloud_column_optical_depth(p);
    let v = exp(-CLOUD_AO_DIFFUSIVITY * tau);
    // Occluded part: cloud bases lit by diffusely transmitted sunlight (Lambertian, PI * L).
    return sky * v + (1.0 - v) * PI * e_top / (1.0 + 0.1125 * tau);
}
const CLOUD_AO_DIFFUSIVITY: f32 = 1.66;

/// Blurred cloud column optical depth above an atmosphere-space point (cloud AO).
@if(CLOUDS_ENABLED)
fn cloud_column_optical_depth(p: vec3<f32>) -> f32 {
    let n = normalize(p);
    var tau = -log(max(cloud_far_transmittance(p, n, true), 1e-6));
    @if(CLOUD_FIELD_BOUND) {
        let uvw = cloud_field_uvw(p, textureLoad(cloud_field_sdf, vec3(0), 0).ba);
        if (all(uvw.xy >= vec2(0.0)) && all(uvw.xy <= vec2(1.0))) {
            let field = textureSampleLevel(cloud_field_sdf, atmosphere_lut_sampler, vec3(uvw.xy, 0.5), 0.0).g;
            // Fade to the march over the outer 10 % of the field.
            let e = min(min(uvw.x, 1.0 - uvw.x), min(uvw.y, 1.0 - uvw.y));
            tau = mix(tau, field * settings.cloud_shadow_map_strength, saturate(e / 0.1));
        }
    }
    return tau;
}
const GROUND_SKY_DIRS: u32 = 6u;
const GROUND_SKY_STEPS: u32 = 6u;
fn ground_sky_irradiance(p: vec3<f32>, pixel_coords: vec2<f32>) -> vec3<f32> {
    let n = normalize(p);
    let r = length(p);
    let tx = normalize(select(vec3(1.0, 0.0, 0.0), vec3(0.0, 0.0, 1.0), abs(n.x) > 0.9) - n * dot(n, select(vec3(1.0, 0.0, 0.0), vec3(0.0, 0.0, 1.0), abs(n.x) > 0.9)));
    let ty = cross(n, tx);
    var sum = vec3(0.0);
    @if(!SKY_VIEW_LUT_PASS) {
    // Cheap path: 8 fetches of this frame's sky-view LUT (sky as seen from the camera, a good
    // proxy for nearby ground). The 64-sample march below cost 3-5 ms of render_sky.
    let r_cam = length(get_view_position());
    for (var k = 0u; k < GROUND_SKY_DIRS; k++) {
        let u = (f32(k) + 0.5) / f32(GROUND_SKY_DIRS);
        let phi = f32(k) * 2.399963;
        let st = sqrt(u);
        let ct = sqrt(1.0 - u);
        let d = normalize(n * ct + (tx * cos(phi) + ty * sin(phi)) * st);
        sum += sample_sky_view_lut(r_cam, direction_world_to_atmosphere(d));
    }
    return PI * sum / f32(GROUND_SKY_DIRS);
    }
    for (var k = 0u; k < GROUND_SKY_DIRS; k++) {
        let u = (f32(k) + 0.5) / f32(GROUND_SKY_DIRS);
        let phi = f32(k) * 2.399963; // golden angle
        let st = sqrt(u);
        let ct = sqrt(1.0 - u);
        let d = normalize(n * ct + (tx * cos(phi) + ty * sin(phi)) * st);
        let t_end = distance_to_top_atmosphere_boundary(atmosphere, r, ct);
        var tr = vec3(1.0);
        var l = vec3(0.0);
        for (var i = 0u; i < GROUND_SKY_STEPS; i++) {
            let ua = f32(i) / f32(GROUND_SKY_STEPS);
            let ub = f32(i + 1u) / f32(GROUND_SKY_STEPS);
            let ta = t_end * ua * ua;
            let tb = t_end * ub * ub;
            let q = p + d * (0.5 * (ta + tb));
            let rq = length(q);
            let scattering = sample_density_lut(rq, SCATTERING_DENSITY);
            let extinction = sample_density_lut(rq, ABSORPTION_DENSITY) + scattering;
            let st_i = exp(-extinction * (tb - ta));
            let ins = sample_local_inscattering(scattering, d, q, pixel_coords);
            l += tr * (ins - ins * st_i) / max(extinction, MIN_EXTINCTION);
            tr *= st_i;
        }
        sum += l;
    }
    return PI * sum / f32(GROUND_SKY_DIRS);
}

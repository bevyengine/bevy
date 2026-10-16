// Cloud rendering functions using 3D FBM noise

import bevy_render::maths::{PI, ray_sphere_intersect};
import package::render::utils::interleaved_gradient_noise;
import package::atmosphere::{
    types::{Atmosphere, AtmosphereSettings},
    bindings::{settings, atmosphere},
};

struct CloudLayer {
    cloud_layer_start: f32,
    cloud_layer_end: f32,
    cloud_density: f32,
    cloud_absorption: f32,
    cloud_scattering: f32,
    noise_scale: f32,
    noise_offset: vec3<f32>,
    detail_noise_scale: f32,
    detail_strength: f32,
    wind: vec3<f32>,
    profile_bottom_exponent: f32,
    profile_top_exponent: f32,
    profile_top_min: f32,
}

@group(0) @binding(20) var<uniform> cloud_layer: CloudLayer;
@group(0) @binding(21) var noise_texture_2d: texture_2d<f32>;
@group(0) @binding(22) var noise_sampler_3d: sampler;
@group(0) @binding(24) var perlin_worley_noise_3d: texture_3d<f32>;
@group(0) @binding(33) var detail_worley_noise_3d: texture_3d<f32>;

fn safe_normalize(v: vec3<f32>) -> vec3<f32> {
    let l2 = dot(v, v);
    if (l2 <= 1e-12) {
        return vec3(0.0, 1.0, 0.0);
    }
    return v * inverseSqrt(l2);
}

// Sample weather map (RGBA) at a given world-space scale.
// - R: coverage
// - G: cloud type (0 stratus → 0.5 stratocumulus → 1 cumulus)
// - B: density scale
// - A: spare
/// Warped cloud-shadow-map coordinates: light-space x/extent = a s + (1 - a) s^3, s = 2 uv - 1.
/// Texels are a x the linear size at the anchor and (3 - 2a) x at the border, a single-map "near
/// cascade": looking along the light, near haze and ground shadows read one texel per screen
/// region, and uniform 125 m texels showed as straight-edged patches. The cubic keeps the fine
/// texels further out than a quadratic.
const CLOUD_SHADOW_WARP_A: f32 = 0.125;
fn cloud_shadow_uv_to_xy(uv: vec2<f32>, extent: f32) -> vec2<f32> {
    let s = 2.0 * uv - 1.0;
    return extent * (CLOUD_SHADOW_WARP_A * s + (1.0 - CLOUD_SHADOW_WARP_A) * s * s * s);
}
fn cloud_shadow_xy_to_uv(xy: vec2<f32>, extent: f32) -> vec2<f32> {
    // s^3 + p s + q = 0, p > 0: the hyperbolic form of the single real root is stable near
    // s = 0 (Cardano's difference of cube roots cancelled there, so uv -> xy -> uv drifted and
    // the temporal reprojection crawled).
    let p = CLOUD_SHADOW_WARP_A / (1.0 - CLOUD_SHADOW_WARP_A);
    let q = -(xy / extent) / (1.0 - CLOUD_SHADOW_WARP_A);
    let k = 2.0 * sqrt(p / 3.0);
    let s = -k * sinh(asinh(1.5 * q / p * sqrt(3.0 / p)) / 3.0);
    return 0.5 * s + 0.5;
}

fn sample_weather(world_pos: vec3<f32>, weather_scale: f32) -> vec4<f32> {
    let rot = mat2x2<f32>(
        0.8660254, -0.5,
        0.5, 0.8660254
    ); // 30 degrees
    let xz = rot * world_pos.xz;
    let uv = (xz + cloud_layer.noise_offset.xz) / max(1.0, weather_scale);
    var w = textureSampleLevel(noise_texture_2d, noise_sampler_3d, uv, 0.0);
    // Mesoscale cloud fraction from the A channel at 4x the tile (64/32/16 km): erodes coverage
    // (Nubis ValueErosion form), keeping it in cloudy regions and clearing gaps between them.
    let meso = textureSampleLevel(noise_texture_2d, noise_sampler_3d, uv * 0.25 + vec2(0.37, 0.61), 0.0).a;
    let erosion = 1.0 - smoothstep(0.30, 0.65, meso);
    w.r = saturate((w.r - erosion) / max(1.0 - erosion, 1e-3));
    return w;
}

fn sample_shape_noise(world_pos: vec3<f32>, h: f32, mip: f32) -> vec4<f32> {
    let rot = mat2x2<f32>(
        0.8660254, -0.5,
        0.5, 0.8660254
    );
    let xz = rot * (world_pos.xz + cloud_layer.noise_offset.xz);
    // Shape tiles at noise_scale; height uses normalized layer height + slow vertical scroll.
    // Isotropic period in meters (Nubis: 128³ shape tile spans a few km). Using normalized
    // layer height for w made the vertical period ≈ layer thickness while xz spanned 16 km.
    let period = max(1.0, cloud_layer.noise_scale * 0.125);
    let uv = xz / period;
    let w = (h * cloud_layer_thickness_m() + cloud_layer.noise_offset.y) / period;
    return textureSampleLevel(perlin_worley_noise_3d, noise_sampler_3d, vec3(uv, w), mip);
}

fn sample_detail_noise(world_pos: vec3<f32>, h: f32, mip: f32) -> vec4<f32> {
    let rot = mat2x2<f32>(
        0.8660254, -0.5,
        0.5, 0.8660254
    );
    let xz = rot * (world_pos.xz + cloud_layer.noise_offset.xz * 1.7);
    let period = max(1.0, cloud_layer.detail_noise_scale);
    let uv = xz / period;
    let w = (h * cloud_layer_thickness_m() + cloud_layer.noise_offset.y * 2.0) / period;
    return textureSampleLevel(detail_worley_noise_3d, noise_sampler_3d, vec3(uv, w), mip);
}

fn cloud_layer_thickness_m() -> f32 {
    return max(1.0, cloud_layer.cloud_layer_end - cloud_layer.cloud_layer_start);
}


/// Get cloud scattering coefficient per unit density
fn get_cloud_scattering_coeff() -> f32 {
    return cloud_layer.cloud_scattering;
}

/// Get cloud absorption coefficient per unit density
fn get_cloud_absorption_coeff() -> f32 {
    return cloud_layer.cloud_absorption;
}

// PERF/DEBUG: Hardcoded repeating sphere SDF volume (no texture fetches).
const CELL_SIZE_M: f32 = 4000.0;
const RADIUS_M: f32 = 750.0;
const SOFTNESS_M: f32 = 10.0;
const CLOUD_DEBUG_SINGLE_SPHERE: bool = false;
// Primary cloud shape toggle (debug spheres vs Nubis density). Keep a debug path (roadmap 2e).
const CLOUD_USE_NOISE_SHAPE: bool = true;
const CLOUD_DEBUG_GRADIENT_SUBSTEPS: bool = false;

fn debug_sphere_center() -> vec3<f32> {
    let center_r = 0.5 * (cloud_layer.cloud_layer_start + cloud_layer.cloud_layer_end);
    return vec3(0.0, center_r, 0.0);
}

fn remap(x: f32, a: f32, b: f32, c: f32, d: f32) -> f32 {
    let t = clamp((x - a) / max(1e-6, b - a), 0.0, 1.0);
    return mix(c, d, t);
}

/// Dimensional profile (Nubis Evolved 2022 sky model: vertical_profile * coverage) with a
/// parameterized vertical envelope, h' = height / cloud-top height of the column:
///   vertical_profile = norm * h'^a * (1 - h')^b,  norm = 1 / max (peak 1 at h' = a / (a + b)).
/// a = profile_bottom_exponent, b = profile_top_exponent (`CloudLayer`):
///  - physical default a = b = 1: adiabatic liquid water grows linearly above the flat
///    condensation base, and entrainment lowers the adiabatic fraction ~linearly toward the top
///    (cloud-physics texts, e.g. Rogers & Yau, A Short Course in Cloud Physics, ch. 2-3);
///  - Nubis Evolved envelope model (2022 p97): a = 2, b = 1.5.
/// The column's cloud top rises with coverage from profile_top_min to the layer top (Nubis:
/// cloud height follows coverage), so cells are domed rather than extruded.
fn cloud_dimensional_profile(h: f32, coverage: f32, cloud_type: f32) -> f32 {
    let a = cloud_layer.profile_bottom_exponent;
    let b = cloud_layer.profile_top_exponent;
    let top = mix(cloud_layer.profile_top_min, 1.0, coverage);
    let x = h / max(top, 1e-3);
    if (x >= 1.0 || coverage <= 0.0) {
        return 0.0;
    }
    let peak = a / max(a + b, 1e-6);
    let max_v = max(pow(peak, a) * pow(1.0 - peak, b), 1e-6);
    let vertical_profile = pow(x, a) * pow(1.0 - x, b) / max_v;
    return saturate(vertical_profile * coverage);
}

/// Coarse density context: weather + dimensional profile (Nubis 2022 vertical_profile *
/// coverage) at the same sample position as the fine density (sample_cumulus_shape_lod).
/// Returns (dimensional_profile, density_scale^4, h, cloud_type).
fn cloud_density_context(r: f32, world_pos: vec3<f32>) -> vec4<f32> {
    let layer_thickness = cloud_layer.cloud_layer_end - cloud_layer.cloud_layer_start;
    let height_in_layer = r - cloud_layer.cloud_layer_start;
    let h = clamp(height_in_layer / max(1.0, layer_thickness), 0.0, 1.0);

    // Must match the fine density's sample position: the coarse density is the empty-space
    // predicate (ray march, SDF seeds).
    let sample_pos = cloud_sample_pos(world_pos);
    let weather = sample_weather(sample_pos, cloud_layer.noise_scale);
    var coverage = clamp(weather.r, 0.0, 1.0);
    let cloud_type = clamp(weather.g, 0.0, 1.0);
    let density_scale = pow(clamp(weather.b, 0.0, 1.0), 4.0);
    let dimensional_profile = cloud_dimensional_profile(h, coverage, cloud_type);
    // Return profile, density_scale, h, cloud_type (sample_pos reconstructed by callers).
    return vec4(dimensional_profile, density_scale, h, cloud_type);
}

/// Noise sample position. Nubis (2022 sky and 2023 voxel samplers) only scrolls the noise with
/// the wind (noise_offset, applied in the samplers); there is no height skew and no curl
/// displacement of the position. The curl turbulence lives in the detail noise ("curly
/// alligator", Nubis Cubed; HZD15 also curls only the detail noise).
fn cloud_sample_pos(world_pos: vec3<f32>) -> vec3<f32> {
    return world_pos;
}

/// Coarse density: weather + vertical profile only (no 3D noise), an upper bound of the fine
/// density (`sample_cumulus_shape_lod`) used as the empty-space predicate (ray march, SDF seeds).
/// The fine density is at most (profile * scale^4)^lerp(0.3, 0.6, scale^4) (noise <= 1, erosion
/// only removes); the bound must carry the same power, or thin low-density-scale clouds, which
/// the power lifts by orders of magnitude (2e-4 -> 0.08), fall under the empty threshold.
fn sample_cumulus_shape_coarse(r: f32, world_pos: vec3<f32>) -> f32 {
    let ctx = cloud_density_context(r, world_pos);
    let x = saturate(ctx.x * ctx.y);
    return select(0.0, pow(x, mix(0.3, 0.6, max(1e-4, ctx.y))), x > 0.0);
}

/// Fine density with mip LOD (design.md §1.3 / §1.4).
fn sample_cumulus_shape_lod(r: f32, world_pos: vec3<f32>, mip: f32) -> f32 {
    let layer_thickness = cloud_layer.cloud_layer_end - cloud_layer.cloud_layer_start;
    let height_in_layer = r - cloud_layer.cloud_layer_start;
    let h = clamp(height_in_layer / max(1.0, layer_thickness), 0.0, 1.0);
    let sample_pos = cloud_sample_pos(world_pos);

    let weather = sample_weather(sample_pos, cloud_layer.noise_scale);
    let coverage = clamp(weather.r, 0.0, 1.0);
    // Nubis detail type (0 wispy, 1 billowy) and density scale: authored weather fields.
    let detail_type = clamp(weather.g, 0.0, 1.0);
    let density_scale = clamp(weather.b, 0.0, 1.0);
    // Nubis Evolved (2022) sky model: dimensional_profile = vertical_profile * coverage.
    let base_profile = cloud_dimensional_profile(h, coverage, detail_type);
    if (base_profile <= 0.0) {
        return 0.0;
    }

    // Large-scale shape from the 2022 noise (low-frequency Perlin-Worley eroded by Worley fBm,
    // HZD15: fbm = 0.625 lo + 0.25 med + 0.125 hi, remap(pw, fbm - 1, 1, 0, 1)), applied as the
    // 2022 sky density: saturate(noise - (1 - dimensional_profile)).
    let pw = sample_shape_noise(sample_pos, h, mip);
    let wfbm = pw.y * 0.625 + pw.z * 0.25 + pw.w * 0.125;
    let shape_noise = saturate(remap(pw.x, wfbm - 1.0, 1.0, 0.0, 1.0));
    let profile = saturate(shape_noise - (1.0 - base_profile));

    // Nubis Cubed (2023) up-res of that profile (Guerrilla's VoxelCloudSampler,
    // GetUprezzedVoxelCloudDensity): wispy = lerp(R, G, profile), billowy = lerp(B, A,
    // profile^0.25) * 0.3, composite = lerp(wispy, billowy, detail_type), value erosion of the
    // profile by the composite, density scale^4, sharpened by pow(d, lerp(0.3, 0.6, scale^4)).
    // Nubis's extra near-camera density fade (x0.25 at 10 m, for player visibility) is not
    // applied: density does not depend on the camera.
    // Detail LOD from the sample footprint (anti-aliasing and cache): `mip` is log2(1 + d / 4 km)
    // (shape noise); a half-res pixel spans ~2.2 mrad, and a detail texel is detail_noise_scale /
    // 128, so the footprint LOD is mip + log2(4000 m * 2.2e-3 / texel).
    let detail_texel = max(cloud_layer.detail_noise_scale, 1.0) / 128.0;
    let n = sample_detail_noise(sample_pos, h, mip + max(log2(4000.0 * 2.2e-3 / detail_texel), 0.0));
    let wispy = mix(n.r, n.g, profile);
    let billowy = mix(n.b * 0.3, n.a * 0.3, pow(profile, 0.25));
    var composite = mix(wispy, billowy, detail_type) * cloud_layer.detail_strength;
    // Near-camera high-frequency detail (Nubis Cubed sampler, inHFDetails): the highest-frequency
    // wispy (G) and billowy (A) noise folded twice over 0, blended in within 150 m of the camera.
    // The camera distance follows from the shape LOD, mip = log2(1 + d / 4 km).
    let cam_dist = (exp2(mip) - 1.0) * 4000.0;
    let hhf = saturate(mix(1.0 - pow(abs(abs(n.g * 2.0 - 1.0) * 2.0 - 1.0), 4.0),
        pow(abs(abs(n.a * 2.0 - 1.0) * 2.0 - 1.0), 2.0), detail_type));
    let hf_blend = saturate((cam_dist - 50.0) / 100.0);
    composite = mix(hhf, composite, mix(0.9, 1.0, hf_blend));
    var dens = saturate((profile - composite) / max(1.0 - composite, 1e-4));
    let powered_scale = pow(density_scale, 4.0);
    dens *= powered_scale;
    dens = pow(dens, mix(0.3, 0.6, max(1e-4, powered_scale)));
    dens = pow(dens, mix(0.5, 1.0, hf_blend)) * mix(0.666, 1.0, hf_blend);
    return dens;
}

fn sample_cumulus_shape(r: f32, world_pos: vec3<f32>) -> f32 {
    return sample_cumulus_shape_lod(r, world_pos, 0.0);
}

/// Cloud shape / coverage term in [0, 1].
/// This is *not* a physical density by itself: it is the normalized field used to
/// shape the cloud volume (noise + height falloff).
fn get_cloud_coverage(r: f32, world_pos: vec3<f32>) -> f32 {
    // Check if we're within the cloud layer
    if (r < cloud_layer.cloud_layer_start || r > cloud_layer.cloud_layer_end) {
        return 0.0;
    }

    if (CLOUD_USE_NOISE_SHAPE) {
        return sample_cumulus_shape(r, world_pos);
    }

    // Debug sphere volume path
    if (CLOUD_DEBUG_SINGLE_SPHERE) {
        let c = debug_sphere_center();
        let sdf = length(world_pos - c) - RADIUS_M;
        let density = 1.0 - smoothstep(-SOFTNESS_M, SOFTNESS_M, sdf);
        return clamp(density, 0.0, 1.0);
    }

    // Repeating spheres in XZ only.
    let layer_thickness = cloud_layer.cloud_layer_end - cloud_layer.cloud_layer_start;
    let height_in_layer = r - cloud_layer.cloud_layer_start;
    let center_y = 0.5 * layer_thickness;
    let y_rel = height_in_layer - center_y;
    let cell_xz = fract(world_pos.xz / CELL_SIZE_M) - vec2(0.5);
    let q = vec3(cell_xz * CELL_SIZE_M, y_rel);
    let sdf = length(q) - RADIUS_M;
    let sphere_density = 1.0 - smoothstep(-SOFTNESS_M, SOFTNESS_M, sdf);
    return clamp(sphere_density, 0.0, 1.0);
}

/// Returns (density, grad_mag) for the current debug cloud field.
/// - density is the normalized coverage term in [0,1]
/// - grad_mag is an estimate of |∇density| in 1/m, useful for adaptive stepping.
fn sample_cloud_field_density_and_grad(r: f32, world_pos: vec3<f32>) -> vec2<f32> {
    // Outside cloud layer => empty and flat field.
    if (r < cloud_layer.cloud_layer_start || r > cloud_layer.cloud_layer_end) {
        return vec2(0.0, 0.0);
    }

    // Noise shape: skip expensive finite-diff gradient unless debug flag is on (roadmap 2e).
    if (CLOUD_USE_NOISE_SHAPE) {
        let d0 = sample_cumulus_shape(r, world_pos);
        if (!CLOUD_DEBUG_GRADIENT_SUBSTEPS) {
            return vec2(d0, 0.0);
        }
        const EPS_M: f32 = 500.0;
        let up = safe_normalize(world_pos);
        let east = safe_normalize(vec3(-up.z, 0.0, up.x));
        let north = safe_normalize(cross(up, east));
        let dx = sample_cumulus_shape(r, world_pos + east * EPS_M);
        let dz = sample_cumulus_shape(r, world_pos + north * EPS_M);
        let ddx = abs(dx - d0) / EPS_M;
        let ddz = abs(dz - d0) / EPS_M;
        let grad_mag = sqrt(ddx * ddx + ddz * ddz);
        return vec2(d0, grad_mag);
    }

    // Debug sphere field: analytic gradient proxy from SDF smoothstep.
    var sdf: f32;
    if (CLOUD_DEBUG_SINGLE_SPHERE) {
        let c = debug_sphere_center();
        sdf = length(world_pos - c) - RADIUS_M;
    } else {
        let layer_thickness = cloud_layer.cloud_layer_end - cloud_layer.cloud_layer_start;
        let height_in_layer = r - cloud_layer.cloud_layer_start;
        let center_y = 0.5 * layer_thickness;
        let y_rel = height_in_layer - center_y;
        let cell_xz = fract(world_pos.xz / CELL_SIZE_M) - vec2(0.5);
        let q = vec3(cell_xz * CELL_SIZE_M, y_rel);
        sdf = length(q) - RADIUS_M;
    }

    let edge0 = -SOFTNESS_M;
    let edge1 = SOFTNESS_M;
    let denom = (edge1 - edge0);
    let tt = clamp((sdf - edge0) / denom, 0.0, 1.0);
    let smoothen = tt * tt * (3.0 - 2.0 * tt);
    let sphere_density = 1.0 - smoothen;
    let d_sphere_density_dsdf = 6.0 * tt * (1.0 - tt) / denom;
    let density = clamp(sphere_density, 0.0, 1.0);
    let grad_mag = d_sphere_density_dsdf;
    return vec2(density, grad_mag);
}

/// Returns (start_t, end_t, valid) for the segment of the ray inside the cloud layer shell.
/// - start_t/end_t are distances along the ray direction (meters), relative to ray_origin.
/// - valid is 1.0 if the segment exists, 0.0 otherwise.
fn cloud_layer_segment(ray_origin: vec3<f32>, ray_dir: vec3<f32>) -> vec3<f32> {
    // IMPORTANT: do the ray/sphere intersection math in kilometers to improve numerical stability.
    // In meters, `r` is ~6.3e6 and for grazing angles the discriminant can become ill-conditioned,
    // producing "banded" invalid intersections as the sun elevation changes.
    // Doing the math in km keeps magnitudes ~6.3e3 and greatly reduces cancellation.
    const M_TO_KM: f32 = 0.001;
    const KM_TO_M: f32 = 1000.0;

    let r_km = length(ray_origin) * M_TO_KM;
    let up = normalize(ray_origin);
    let mu = dot(ray_dir, up);

    let bottom_radius_km = cloud_layer.cloud_layer_start * M_TO_KM;
    let top_radius_km = cloud_layer.cloud_layer_end * M_TO_KM;

    // Unreal-style intersection selection:
    // Compute intersections with the top and bottom spheres, then derive a single [TMin, TMax]
    // interval from carefully selected roots. This avoids sign/branch issues at grazing angles.
    let t_top = ray_sphere_intersect(r_km, mu, top_radius_km);
    if (t_top.x < 0.0 && t_top.y < 0.0) {
        return vec3(0.0, 0.0, 0.0);
    }
    let t_bottom = ray_sphere_intersect(r_km, mu, bottom_radius_km);

    var t_min = t_top.x;
    var t_max = t_top.y;

    // If we also intersect the bottom sphere, combine both.
    if (!(t_bottom.x < 0.0 && t_bottom.y < 0.0)) {
        // If we see both intersections in front of us, keep the min/closest, otherwise the max/furthest.
        var temp_top = select(max(t_top.x, t_top.y), min(t_top.x, t_top.y), (t_top.x > 0.0) && (t_top.y > 0.0));
        var temp_bottom = select(max(t_bottom.x, t_bottom.y), min(t_bottom.x, t_bottom.y), (t_bottom.x > 0.0) && (t_bottom.y > 0.0));

        if ((t_bottom.x > 0.0) && (t_bottom.y > 0.0)) {
            // If we can see the bottom of the layer, make sure we use the camera (0) or the closest top intersection.
            temp_top = max(0.0, min(t_top.x, t_top.y));
        }

        t_min = min(temp_bottom, temp_top);
        t_max = max(temp_bottom, temp_top);
    }

    t_min = max(0.0, t_min);
    t_max = max(0.0, t_max);
    let valid = (t_max > t_min);

    // Convert km back to meters for callers.
    return vec3(t_min * KM_TO_M, t_max * KM_TO_M, select(0.0, 1.0, valid));
}

/// Cloud *medium density* used for extinction / scattering integration.
/// This is the normalized coverage term scaled by `cloud_layer.cloud_density`.
fn get_cloud_medium_density(r: f32, world_pos: vec3<f32>) -> f32 {
    return get_cloud_coverage(r, world_pos) * cloud_layer.cloud_density;
}

fn get_cloud_medium_density_coarse(r: f32, world_pos: vec3<f32>) -> f32 {
    if (r < cloud_layer.cloud_layer_start || r > cloud_layer.cloud_layer_end) {
        return 0.0;
    }
    if (!CLOUD_USE_NOISE_SHAPE) {
        return get_cloud_medium_density(r, world_pos);
    }
    return sample_cumulus_shape_coarse(r, world_pos) * cloud_layer.cloud_density;
}

fn get_cloud_medium_density_lod(r: f32, world_pos: vec3<f32>, mip: f32) -> f32 {
    if (r < cloud_layer.cloud_layer_start || r > cloud_layer.cloud_layer_end) {
        return 0.0;
    }
    if (!CLOUD_USE_NOISE_SHAPE) {
        return get_cloud_medium_density(r, world_pos);
    }
    return sample_cumulus_shape_lod(r, world_pos, mip) * cloud_layer.cloud_density;
}

fn cloud_mip_from_distance(dist_m: f32) -> f32 {
    // Nubis3: mip = log2(1 + dist · k). k chosen so ~mip 0 near, ~mip 4 at ~16 km.
    @if(ATM_DBG_MIP0) { return 0.0; }
    return log2(1.0 + dist_m * (1.0 / 4000.0));
}

/// Raymarch through clouds towards the sun to compute volumetric shadow
/// Returns the light transmittance factor [0,1] where 0 = fully shadowed, 1 = no shadow
/// Properly handles viewer inside clouds and grazing angles
fn compute_cloud_shadow(
    world_pos: vec3<f32>,
    sun_dir: vec3<f32>,
    steps: u32,
    pixel_coords: vec2<f32>,
) -> f32 {
    // Early exit if clouds are disabled
    if (cloud_layer.cloud_density <= 0.0) {
        return 1.0;
    }

    // March bounds: only integrate within the cloud layer shell along the sun ray.
    let seg = cloud_layer_segment(world_pos, sun_dir);
    if (seg.z < 0.5) {
        return 1.0;
    }

    var march_start = max(seg.x, 0.0);
    var march_end = seg.y;
    if (march_start >= march_end || march_end <= 0.0) {
        return 1.0;
    }

    // Earth (planet) shadow term:
    // If the sun ray hits the planet surface before it exits the cloud layer,
    // the sun is fully occluded and there is no direct lighting.
    let r0 = length(world_pos);
    let up0 = normalize(world_pos);
    let mu0 = dot(sun_dir, up0);
    let ground_i = ray_sphere_intersect(r0, mu0, atmosphere.inner_radius);
    // `ground_i.x` is the nearest positive intersection along the ray, if any.
    if (ground_i.x > 0.0 && ground_i.x < march_end) {
        return 0.0;
    }

    let march_distance = march_end - march_start;

    // Adaptive step count: long segments need more samples, otherwise we can miss small dense regions
    // (especially with the repeating-sphere debug density field).
    const TARGET_STEP_M: f32 = 1500.0;
    const MAX_SHADOW_STEPS: f32 = 16.0;
    let desired = clamp(ceil(march_distance / TARGET_STEP_M), f32(steps), MAX_SHADOW_STEPS);
    let shadow_steps = max(1u, u32(desired));

    let step_size = march_distance / f32(shadow_steps);
    var optical_depth = 0.0;

    // Raymarch through clouds towards sun with per-step jitter.
    for (var i = 0u; i < shadow_steps; i++) {
        let j = interleaved_gradient_noise(pixel_coords, 100u + i); // [0,1]
        let t = march_start + (f32(i) + j) * step_size;
        let sample_pos = world_pos + sun_dir * t;
        let sample_r = length(sample_pos);

        let density = get_cloud_medium_density(sample_r, sample_pos);
        if (density > 0.0) {
            let extinction = density * (cloud_layer.cloud_scattering + cloud_layer.cloud_absorption);
            optical_depth += extinction * step_size;
            // Early out when essentially fully shadowed
            if (optical_depth > 8.0) {
                return 0.0;
            }
        }
    }

    return exp(-optical_depth);
}

/// Simplified cloud contribution for a single sample point
/// Returns (luminance_added, transmittance_multiplier)
fn sample_cloud_contribution(
    world_pos: vec3<f32>,
    step_size: f32,
) -> vec2<f32> {
    let r = length(world_pos);
    let density = get_cloud_medium_density(r, world_pos);
    
    if (density < 0.01) {
        return vec2(0.0, 1.0);
    }
    
    // Physically correct coefficients (units: m^-1 per unit density)
    let extinction = density * (cloud_layer.cloud_scattering + cloud_layer.cloud_absorption);
    let scattering = density * cloud_layer.cloud_scattering;
    
    // Beer's law
    let transmittance = exp(-extinction * step_size);
    
    // Simple uniform scattering (could be enhanced with actual sun direction)
    let in_scatter = scattering * (1.0 - transmittance);
    
    return vec2(in_scatter, transmittance);
}

// ---------------------------------------------------------------------------------------------
// Coarse cloud signed-distance field (Nubis Cubed, Advances 2023: "SDF", 512x512x64 voxels,
// -256 m .. 4096 m; GetVoxelCloudDistance feeds the secondary-scattering remap and SDF ray
// marching). Here it is generated each frame from the coarse (weather + height profile + shape)
// density over a camera-centred grid (`cloud_field.wesl`): x/z horizontal in atmosphere space,
// z slices span the cloud layer radially.
const CLOUD_FIELD_RES_XY: u32 = 128u;
const CLOUD_FIELD_RES_Z: u32 = 32u;
const CLOUD_FIELD_EXTENT_M: f32 = 24000.0;

fn cloud_field_voxel_m() -> f32 {
    return CLOUD_FIELD_EXTENT_M / f32(CLOUD_FIELD_RES_XY);
}

/// Grid origin (min corner, atmosphere-space x/z), snapped to whole voxels so it does not swim.
/// Field cell of the camera: integer voxel index (stored in the field so consumers use the
/// origin the field was actually built with; the build is spread over several frames).
fn cloud_field_cell(cam: vec3<f32>) -> vec2<f32> {
    return floor(cam.xz / cloud_field_voxel_m());
}

fn cloud_field_origin_from_cell(cell: vec2<f32>) -> vec2<f32> {
    return cell * cloud_field_voxel_m() - 0.5 * CLOUD_FIELD_EXTENT_M;
}

fn cloud_field_origin(cam: vec3<f32>) -> vec2<f32> {
    return cloud_field_origin_from_cell(cloud_field_cell(cam));
}

/// Normalised field coordinate of an atmosphere-space point; outside [0,1]^3 = not covered.
/// `cell` is the stored build cell (`cloud_field_sdf.ba`).
fn cloud_field_uvw(p: vec3<f32>, cell: vec2<f32>) -> vec3<f32> {
    let uv = (p.xz - cloud_field_origin_from_cell(cell)) / CLOUD_FIELD_EXTENT_M;
    let w = (length(p) - cloud_layer.cloud_layer_start)
        / max(cloud_layer.cloud_layer_end - cloud_layer.cloud_layer_start, 1.0);
    return vec3(uv, w);
}

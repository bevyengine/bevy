// Dedicated low-resolution volumetric cloud raymarch (design.md §1.2, roadmap step 1b).
//
// Traces 1 of every 2x2 half-res pixels per frame (checkerboard order from `view.frame_count`,
// RDR2 slides 51-55); `clouds_reconstruct.wesl` fills the rest from reprojected history.
// Runs at a fraction of the view resolution (half res by default) as its own compute pass, so the
// cloud cost is decoupled from `render_sky` and from the atmosphere `AtmosphereMode`.
// Outputs:
// - `out_clouds` RGBA16F: rgb = cloud in-scattered luminance (pre-exposed, *without* aerial
//   perspective), a = cloud transmittance (wavelength independent).
// - `out_clouds_depth` R32F: transmittance-weighted cloud distance in meters (FB16 §5.9.1, eq. 21),
//   0 when the ray sees no cloud. `render_sky` applies aerial perspective once at this distance.

import bevy_render::maths::{PI, ray_sphere_intersect};
import package::render::utils::interleaved_gradient_noise;
import package::atmosphere::{
    bindings::{atmosphere, settings, view, lights, stbn_texture},
    bruneton_functions::{ray_intersects_ground, sample_transmittance_lut_segment},
    functions::{
        uv_to_ray_direction, uv_to_ndc, ndc_to_camera_dist, get_view_position,
        sample_transmittance_lut, sample_sky_view_lut, sample_cloud_shadow_map,
        hg_phase, sample_aerial_view_lut,
    },
    clouds::{
        cloud_layer_segment, get_cloud_medium_density_coarse,
        get_cloud_medium_density_lod, cloud_mip_from_distance, get_cloud_scattering_coeff,
        get_cloud_absorption_coeff, cloud_density_context,
    },
};

@group(0) @binding(13) var depth_texture: texture_depth_2d;
@group(0) @binding(26) var out_clouds: texture_storage_2d<rgba16float, write>;
@group(0) @binding(27) var out_clouds_depth: texture_storage_2d<r32float, write>;

// Hard caps for the first version of the pass; these become `VolumetricCloudSettings` (design §3.3).
const CLOUD_MAX_STEPS: f32 = 96.0;
// Distance-adaptive step (Nubis3): near + far_offset · dist / 16384.
const CLOUD_STEP_NEAR_M: f32 = 40.0;
const CLOUD_STEP_FAR_OFFSET_M: f32 = 220.0;
const CLOUD_STEP_FAR_REF_M: f32 = 16384.0;
const CLOUD_COARSE_STEP_SCALE: f32 = 3.0;
const CLOUD_FINE_EMPTY_TO_COARSE: u32 = 10u;
const CLOUD_COARSE_DENSITY_EPS: f32 = 0.0005;
// Grazing rays enter a 1.5-4 km shell ~140-230 km away, so keep the horizon covered.
const CLOUD_MAX_DISTANCE_M: f32 = 350000.0;
const CLOUD_MIN_TRANSMITTANCE: f32 = 0.01;

// Multiple-scattering octaves (Wrenninge 2013; FB16 §5.8 eq. 19-20), with a <= b.
const MS_OCTAVES: u32 = 3u;
const MS_A: f32 = 0.5; // scattering attenuation per octave
const MS_B: f32 = 0.5; // extinction attenuation per octave
const MS_C: f32 = 0.5; // phase eccentricity attenuation per octave
// Short light taps toward the sun for near-surface self-shadowing detail (Nubis3 2:09:52: "the
// first two samples are taken as before"; the cloud shadow map covers the long range).
const LIGHT_TAP_COUNT: u32 = 6u;
const LIGHT_TAP_STEP_M: f32 = 20.0; // 20,40,...,640 m → 1.26 km
const LIGHT_LONG_TAP_M: f32 = 1500.0;
// Sky-irradiance ambient (design.md §1.5 / RDR2 slide 36): replaces the MS-LUT placeholder.
const CLOUD_AMBIENT_SCALE: f32 = 0.35;
const CLOUD_GROUND_ALBEDO: f32 = 0.28;
const CLOUD_DARK_EDGE_STRENGTH: f32 = 0.75;
// Extinction scale for the Lambert back-scatter clamp (RDR2 slide 30).
const CLOUD_BACK_SCATTER_EXT_SCALE: f32 = 2500.0;

// Order in which the 4 pixels of each 2x2 half-res block are traced (shared with reconstruct).
const CHECKER_OFFSETS: array<vec2<u32>, 4> = array(
    vec2(0u, 0u), vec2(1u, 1u), vec2(1u, 0u), vec2(0u, 1u)
);

fn cloud_phase(cos_theta: f32, g_scale: f32) -> f32 {
    // Dual-lobe HG, same lobes as the previous in-atmosphere integration.
    return mix(hg_phase(cos_theta, 0.85 * g_scale), hg_phase(cos_theta, -0.2 * g_scale), 0.5);
}

/// Cheap sky-irradiance stand-in: average a few sky-view LUT directions (upper hemisphere)
/// plus a Lambertian ground bounce (design.md §1.5; RDR2 slide 36 / FB16 §5.5.1).
fn sample_sky_irradiance(r: f32) -> vec3<f32> {
    var sum = sample_sky_view_lut(r, vec3(0.0, 1.0, 0.0));
    let elev = 0.45;
    let dirs = array(
        vec3(1.0, elev, 0.0),
        vec3(-1.0, elev, 0.0),
        vec3(0.0, elev, 1.0),
        vec3(0.0, elev, -1.0),
        vec3(0.707, elev, 0.707),
        vec3(-0.707, elev, -0.707),
    );
    for (var i = 0u; i < 6u; i++) {
        sum += sample_sky_view_lut(r, normalize(dirs[i]));
    }
    return sum / 7.0;
}

fn cloud_ambient_term(sky_irr: vec3<f32>, h: f32, dimensional_profile: f32) -> vec3<f32> {
    // Height blend bottom→top (FB16 §5.5.1) and Nubis3 profile ambient pow(1-profile, 0.5).
    let ground = sky_irr * CLOUD_GROUND_ALBEDO;
    let height_ambient = mix(ground, sky_irr, clamp(h, 0.0, 1.0));
    let profile_term = pow(max(1.0 - dimensional_profile, 0.0), 0.5);
    return height_ambient * profile_term * CLOUD_AMBIENT_SCALE;
}

// Aerial perspective between the camera and the cloud (at the transmittance-weighted depth d),
// shadowed by the clouds (RDR2 / Bauer 2019). The aerial-view LUT holds the unshadowed in-scatter
// integral L(0..t), so each segment's increment L(t_i) - L(t_{i-1}) is already the exact (Frostbite
// energy-conserving) integral of that segment; only the cloud-shadow visibility is assumed constant
// per segment and taken at a blue-noise jittered position. Segments follow the LUT's squared depth
// distribution and scale with distance; the half-res reconstruct accumulates the jitter.
fn cloud_shadowed_haze(uv: vec2<f32>, origin: vec3<f32>, ray_dir: vec3<f32>, d: f32, r: f32, mu: f32, j: f32) -> vec3<f32> {
    let n = u32(clamp(ceil(d / 400.0), 16.0, 64.0));
    var l = vec3(0.0, 1.0, 0.0);
    let has_light = lights.n_directional_lights > 0u;
    if (has_light) {
        l = lights.directional_lights[0].direction_to_light;
    }
    var prev = vec3(0.0);
    var acc = vec3(0.0);
    for (var i = 1u; i <= n; i++) {
        let x = f32(i) / f32(n);
        let xj = (f32(i) - 1.0 + j) / f32(n);
        let cur = sample_aerial_view_lut(uv, d * x * x);
        var vis = 1.0;
        if (has_light) {
            vis = sample_cloud_shadow_map(origin + ray_dir * (d * xj * xj), l, false);
        }
        acc += max(cur - prev, vec3(0.0)) * vis;
        prev = cur;
    }
    // Towards and beyond the end of the aerial-view LUT range, fade to the sky-view in-scattering
    // distributed along the ray like the optical depth (as render_sky did).
    let ap_max = settings.aerial_view_lut_max_distance;
    let w = smoothstep(0.5 * ap_max, ap_max, d);
    if (w > 0.0) {
        let t_full = sample_transmittance_lut(r, mu);
        let t_to = sample_transmittance_lut_segment(r, mu, d);
        let sky = sample_sky_view_lut(r, ray_dir);
        let frac = (1.0 - t_to) / max(vec3(1.0) - t_full, vec3(1e-4));
        // Beyond the LUT the haze is mostly outside the cloud shadow map extent: keep it unshadowed.
        acc = mix(acc, sky * saturate(frac), w);
    }
    return acc;
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let trace_size = textureDimensions(out_clouds);
    if (any(id.xy >= trace_size)) {
        return;
    }
    let full_size_u = textureDimensions(depth_texture);
    let full_size = vec2<f32>(full_size_u);
    // Half-res pixel traced by this invocation this frame.
    let half_size = max(full_size_u / 2u, vec2(1u));
    let half_px = min(id.xy * 2u + CHECKER_OFFSETS[view.frame_count % 4u], half_size - 1u);
    let uv = (vec2<f32>(half_px) + 0.5) / vec2<f32>(half_size);

    // Conservative scene depth over the low-res footprint: the farthest of the covered texels
    // (reverse-Z: smallest depth), so thin occluders don't punch holes into the reconstruction.
    let ratio = 2;
    let base_px = vec2<i32>(half_px) * ratio;
    var depth = 1.0;
    for (var y = 0; y < ratio; y++) {
        for (var x = 0; x < ratio; x++) {
            let p = min(base_px + vec2(x, y), vec2<i32>(full_size) - 1);
            depth = min(depth, textureLoad(depth_texture, p, 0));
        }
    }

    var result = vec4(0.0, 0.0, 0.0, 1.0);
    var weighted_depth = 0.0;

    let ray_dir = uv_to_ray_direction(uv);
    let ray_origin = get_view_position();
    let r = length(ray_origin);
    let mu = dot(ray_dir, normalize(ray_origin));

    let seg = cloud_layer_segment(ray_origin, ray_dir);
    var t_start = seg.x;
    var t_end = min(seg.y, CLOUD_MAX_DISTANCE_M);
    // Early exits: shell miss, planet occlusion, scene depth bound.
    if (ray_intersects_ground(r, mu)) {
        t_end = min(t_end, ray_sphere_intersect(r, mu, atmosphere.inner_radius).x);
    }
    if (depth > 0.0) {
        t_end = min(t_end, ndc_to_camera_dist(vec3(uv_to_ndc(uv), depth)));
    }

    if (seg.z > 0.5 && t_end > t_start) {
        // Per-pixel jitter of the ray start (STBN when bound, IGN otherwise).
        let stbn_dims = textureDimensions(stbn_texture);
        var jitter: f32;
        if (all(stbn_dims > vec2(1u))) {
            let layer = i32(view.frame_count % u32(textureNumLayers(stbn_texture)));
            jitter = textureLoad(stbn_texture, vec2<i32>(half_px) % vec2<i32>(stbn_dims), layer, 0).r;
        } else {
            jitter = interleaved_gradient_noise(vec2<f32>(half_px), view.frame_count % 64u);
        }

        let sigma_s_unit = get_cloud_scattering_coeff();
        let sigma_t_unit = sigma_s_unit + get_cloud_absorption_coeff();
        var transmittance = 1.0;
        var depth_weight_sum = 0.0;
        var depth_sum = 0.0;
        var luminance = vec3(0.0);
        // Sky irradiance once per ray (reduction stand-in); height/profile modulate per sample.
        let sky_irr = sample_sky_irradiance(r);

        // Coarse/fine march (HZD15 / Nubis17 / design.md §1.4): large steps with weather+profile
        // only; on hit, step back and switch to fine with full noise. After N empty fine samples
        // return to coarse. Light taps only in fine mode with density > 0.
        var t = t_start + jitter * CLOUD_STEP_NEAR_M;
        var fine_mode = false;
        var fine_empty: u32 = 0u;
        var step_i: u32 = 0u;
        let max_steps = u32(CLOUD_MAX_STEPS);

        loop {
            if (t >= t_end || step_i >= max_steps || transmittance < CLOUD_MIN_TRANSMITTANCE) {
                break;
            }
            step_i += 1u;

            let dist_step = CLOUD_STEP_NEAR_M
                + CLOUD_STEP_FAR_OFFSET_M * clamp(t / CLOUD_STEP_FAR_REF_M, 0.0, 8.0);
            let dt = select(dist_step, dist_step * CLOUD_COARSE_STEP_SCALE, !fine_mode);
            let p = ray_origin + ray_dir * t;
            let r_p = length(p);
            let mip = cloud_mip_from_distance(t);

            var density = 0.0;
            if (fine_mode) {
                density = get_cloud_medium_density_lod(r_p, p, mip);
                if (density <= 0.0) {
                    fine_empty += 1u;
                    if (fine_empty >= CLOUD_FINE_EMPTY_TO_COARSE) {
                        fine_mode = false;
                        fine_empty = 0u;
                    }
                    t += dt;
                    continue;
                }
                fine_empty = 0u;
            } else {
                let coarse = get_cloud_medium_density_coarse(r_p, p);
                if (coarse > CLOUD_COARSE_DENSITY_EPS) {
                    // Step back and enter fine mode (Nubis17).
                    t = max(t_start, t - dt);
                    fine_mode = true;
                    fine_empty = 0u;
                    continue;
                }
                t += dt;
                continue;
            }

            let sigma_t = density * sigma_t_unit;
            let sigma_s = density * sigma_s_unit;
            let up = p / r_p;
            let dens_ctx = cloud_density_context(r_p, p);
            let dimensional_profile = dens_ctx.x;
            let h_frac = dens_ctx.z;
            let ambient = cloud_ambient_term(sky_irr, h_frac, dimensional_profile);

            var direct_sum = vec3(0.0);
            for (var light_i: u32 = 0u; light_i < lights.n_directional_lights; light_i++) {
                let light = &lights.directional_lights[light_i];
                let l = (*light).direction_to_light;
                let mu_light = dot(l, up);
                let t_atm = sample_transmittance_lut(r_p, mu_light)
                    * f32(!ray_intersects_ground(r_p, mu_light));
                // Local light march through the real (fine) density (Nubis/HZD cone march): 6 taps with
                // doubling step (~1.3 km) + one long tap, so billows self-shadow. The cloud shadow map
                // (coarse density, 4x4 time-sliced, temporally filtered) is only used for the
                // occlusion *beyond* the local march; using it at `p` replaced all per-billow detail
                // with smooth weather-map blobs once it converged.
                var light_od = 0.0;
                var tap_t = 0.0;
                var tap_dt = LIGHT_TAP_STEP_M;
                for (var k = 0u; k < LIGHT_TAP_COUNT; k++) {
                    let pk = p + l * (tap_t + 0.5 * tap_dt);
                    light_od += get_cloud_medium_density_lod(length(pk), pk, mip + 0.5 * f32(k)) * tap_dt;
                    tap_t += tap_dt;
                    tap_dt *= 2.0;
                }
                let p_far = p + l * (tap_t + LIGHT_LONG_TAP_M * 0.5);
                light_od += get_cloud_medium_density_lod(length(p_far), p_far, mip + 3.0) * LIGHT_LONG_TAP_M;
                let light_optical_depth = light_od * sigma_t_unit;
                let p_beyond = p + l * (tap_t + LIGHT_LONG_TAP_M);
                let t_cloud = sample_cloud_shadow_map(p_beyond, l, false) * exp(-light_optical_depth);
                // Beer-Powder dark edges (HZD15 cheap fallback; design.md §1.5).
                let powder = 1.0 - exp(-2.0 * light_optical_depth);
                let dark_edge = mix(1.0, powder, CLOUD_DARK_EDGE_STRENGTH);
                let cos_theta = dot(l, ray_dir);
                var direct = 0.0;
                var a = 1.0;
                var b = 1.0;
                var c = 1.0;
                // Extinction-driven Lambert floor on the phase (RDR2 slide 30).
                let phase_floor = (1.0 / PI) * saturate(sigma_t * CLOUD_BACK_SCATTER_EXT_SCALE);
                for (var o = 0u; o < MS_OCTAVES; o++) {
                    let phase = max(cloud_phase(cos_theta, c), phase_floor);
                    direct += a * pow(t_cloud, b) * phase * dark_edge;
                    a *= MS_A;
                    b *= MS_B;
                    c *= MS_C;
                }
                direct_sum += (*light).color.rgb * t_atm * direct;
            }
            // Ambient is sky irradiance (once), not the MS-LUT placeholder.
            let scattered = (direct_sum + ambient) * sigma_s;

            let step_t = exp(-sigma_t * dt);
            luminance += transmittance * (scattered - scattered * step_t) / max(sigma_t, 1e-9);
            let w = transmittance * (1.0 - step_t);
            depth_sum += w * t;
            depth_weight_sum += w;
            transmittance *= step_t;
            t += dt;
        }

        if (transmittance < CLOUD_MIN_TRANSMITTANCE) {
            luminance /= max(1.0 - transmittance, 1e-4);
            transmittance = 0.0;
        }
        weighted_depth = select(0.0, depth_sum / depth_weight_sum, depth_weight_sum > 0.0);
        // Pre-composite the aerial perspective in front of the cloud here (half-res, jittered,
        // temporally reconstructed) instead of in the full-res render_sky pass:
        //   rgb = T_ap(0..d) * L_c + (1 - T_c) * L_ap_shadowed(0..d)
        var lum = luminance;
        if (transmittance < 0.999 && weighted_depth > 0.0) {
            let j = fract(jitter + f32(view.frame_count % 64u) * 0.61803398875);
            let t_ap = sample_transmittance_lut_segment(r, mu, weighted_depth);
            let haze = cloud_shadowed_haze(uv, ray_origin, ray_dir, weighted_depth, r, mu, j);
            lum = t_ap * luminance + (1.0 - transmittance) * haze;
        }
        result = vec4(lum * view.exposure, transmittance);
    }

    textureStore(out_clouds, id.xy, result);
    textureStore(out_clouds_depth, id.xy, vec4(weighted_depth, 0.0, 0.0, 0.0));
}

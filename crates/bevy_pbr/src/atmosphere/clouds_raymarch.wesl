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
        get_cloud_absorption_coeff, cloud_density_context, cloud_layer,
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
const MS_OCTAVES: u32 = 5u;
const MS_A: f32 = 0.5; // scattering attenuation per octave
const MS_B: f32 = 0.3; // extinction attenuation per octave
const MS_C: f32 = 0.5; // phase eccentricity attenuation per octave
// Short light taps toward the sun for near-surface self-shadowing detail (Nubis3 2:09:52: "the
// first two samples are taken as before"; the cloud shadow map covers the long range).
const LIGHT_TAP_COUNT: u32 = 6u;
const LIGHT_TAP_STEP_M: f32 = 20.0; // 20,40,...,640 m → 1.26 km
const LIGHT_LONG_TAP_M: f32 = 1500.0;
// Sky-irradiance ambient (design.md §1.5 / RDR2 slide 36): replaces the MS-LUT placeholder.
// Extinction scale for diffuse (ambient) light inside the cloud.
const CLOUD_AMBIENT_EXT_SCALE: f32 = 0.05;
const CLOUD_GROUND_ALBEDO: f32 = 0.28;
const CLOUD_DARK_EDGE_STRENGTH: f32 = 0.5;
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

/// Isotropic ambient in-scatter radiance (per unit sigma_s): the average incident radiance over
/// the sphere, i.e. 1/2 sky hemisphere (`sky_rad`, mean sky radiance) attenuated by the cloud
/// above + 1/2 ground hemisphere (`ground_rad`, Lambertian bounce of sun + sky) attenuated by the
/// cloud below (FB16 §5.5.1 top/bottom ambient; RDR2 slide 36 sky + ground). Attenuation uses the
/// in-layer distance to the top/bottom with a reduced, diffusion-like extinction (Wrenninge
/// octave-style `b` scaling) instead of the previous 0.35 x pow(1-profile, 0.5) double darkening.
fn cloud_ambient_term(sky_rad: vec3<f32>, ground_rad: vec3<f32>, h: f32, sigma_t: f32, thickness: f32) -> vec3<f32> {
    let k = sigma_t * CLOUD_AMBIENT_EXT_SCALE;
    let vis_up = exp(-k * (1.0 - h) * thickness);
    let vis_down = exp(-k * h * thickness);
    return 0.5 * (sky_rad * vis_up + ground_rad * vis_down);
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

// ---------------------------------------------------------------------------------------------
// Reference volumetric path tracer (CLOUD_REFERENCE): ground truth for the realtime lighting.
// Same density (get_cloud_medium_density_lod at mip 0), same sigma_s/sigma_t, same dual-lobe HG
// phase (importance sampled exactly), sun through the transmittance LUT, sky from the sky-view
// LUT on escape, Lambertian ground (CLOUD_GROUND_ALBEDO) lit by sun (ratio-tracked through the
// clouds) + sky. Delta tracking for free flights, ratio tracking for sun visibility; unbounded
// multiple scattering with Russian roulette.
struct RefOut { c: vec4<f32>, d: f32 }

var<private> ref_rng: u32;
fn ref_rand() -> f32 {
    ref_rng = ref_rng * 747796405u + 2891336453u;
    var w = ((ref_rng >> ((ref_rng >> 28u) + 4u)) ^ ref_rng) * 277803737u;
    w = (w >> 22u) ^ w;
    return f32(w) * (1.0 / 4294967296.0);
}

fn ref_density(p: vec3<f32>) -> f32 {
    return get_cloud_medium_density_lod(length(p), p, 0.0);
}

// Ratio-tracked transmittance through the cloud layer along `dir` from `p`.
fn ref_transmittance(p: vec3<f32>, dir: vec3<f32>, sigma_max: f32) -> f32 {
    let seg = cloud_layer_segment(p, dir);
    if (seg.z < 0.5) { return 1.0; }
    var t = max(seg.x, 0.0);
    let t_end = min(seg.y, CLOUD_MAX_DISTANCE_M);
    var tr = 1.0;
    for (var i = 0u; i < 4096u; i++) {
        t += -log(1.0 - ref_rand()) / sigma_max;
        if (t >= t_end || tr < 1e-4) { break; }
        tr *= 1.0 - ref_density(p + dir * t);
    }
    return tr;
}

fn ref_sample_hg(g: f32) -> f32 {
    let u = ref_rand();
    if (abs(g) < 1e-3) { return 1.0 - 2.0 * u; }
    let s = (1.0 - g * g) / (1.0 - g + 2.0 * g * u);
    return (1.0 + g * g - s * s) / (2.0 * g);
}

fn ref_scatter_dir(d: vec3<f32>) -> vec3<f32> {
    // Dual lobe = 0.5 HG(0.85) + 0.5 HG(-0.2) (cloud_phase with g_scale 1).
    let g = select(-0.2, 0.85, ref_rand() < 0.5);
    let cos_t = clamp(ref_sample_hg(g), -1.0, 1.0);
    let sin_t = sqrt(max(0.0, 1.0 - cos_t * cos_t));
    let phi = 2.0 * PI * ref_rand();
    let a = select(vec3(1.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0), abs(d.x) > 0.9);
    let x = normalize(cross(a, d));
    let y = cross(d, x);
    return normalize(x * (cos(phi) * sin_t) + y * (sin(phi) * sin_t) + d * cos_t);
}

fn ref_sun_radiance(p: vec3<f32>, l: vec3<f32>, color: vec3<f32>) -> vec3<f32> {
    let r = length(p);
    let mu = dot(l, p / r);
    return color * sample_transmittance_lut(r, mu) * f32(!ray_intersects_ground(r, mu));
}

fn cloud_reference_trace(origin: vec3<f32>, view_dir: vec3<f32>, t0: f32, t1: f32, px: vec2<u32>, sky_irr: vec3<f32>) -> RefOut {
    ref_rng = (px.x * 1973u + px.y * 9277u + view.frame_count * 26699u) | 1u;
    let sigma_s_unit = get_cloud_scattering_coeff();
    let sigma_t_unit = sigma_s_unit + get_cloud_absorption_coeff();
    let albedo = sigma_s_unit / sigma_t_unit;
    let has_light = lights.n_directional_lights > 0u;
    var l = vec3(0.0, 1.0, 0.0);
    var sun_color = vec3(0.0);
    if (has_light) {
        l = lights.directional_lights[0].direction_to_light;
        sun_color = lights.directional_lights[0].color.rgb;
    }

    // Camera-ray transmittance + weighted depth by ratio tracking (low variance alpha).
    var t = t0;
    var tr = 1.0;
    var dsum = 0.0;
    var wsum = 0.0;
    for (var i = 0u; i < 4096u; i++) {
        t += -log(1.0 - ref_rand()) / sigma_t_unit;
        if (t >= t1 || tr < 1e-4) { break; }
        let ntr = tr * (1.0 - ref_density(origin + view_dir * t));
        dsum += (tr - ntr) * t;
        wsum += tr - ntr;
        tr = ntr;
    }

    // Radiance by delta tracking.
    var L = vec3(0.0);
    var thr = vec3(1.0);
    var p = origin;
    var d = view_dir;
    var seg_t0 = t0;
    var seg_t1 = t1;
    for (var bounce = 0u; bounce < 256u; bounce++) {
        var tt = seg_t0;
        var collided = false;
        for (var i = 0u; i < 4096u; i++) {
            tt += -log(1.0 - ref_rand()) / sigma_t_unit;
            if (tt >= seg_t1) { break; }
            if (ref_rand() < ref_density(p + d * tt)) { collided = true; break; }
        }
        if (!collided) {
            if (bounce == 0u) { break; } // camera ray: background handled by compositing
            let r = length(p);
            let mu = dot(d, p / r);
            if (ray_intersects_ground(r, mu)) {
                // Lambertian ground: sun (through clouds) + sky irradiance (pi * mean sky radiance).
                let hit = p + d * ray_sphere_intersect(r, mu, atmosphere.inner_radius).x;
                let n = normalize(hit);
                var e = PI * sky_irr;
                if (has_light) {
                    let ml = max(dot(l, n), 0.0);
                    e += ref_sun_radiance(hit, l, sun_color) * ml * ref_transmittance(hit, l, sigma_t_unit);
                }
                L += thr * CLOUD_GROUND_ALBEDO / PI * e;
            } else {
                L += thr * sample_sky_view_lut(r, d);
            }
            break;
        }
        p = p + d * tt;
        thr *= albedo;
        // Next-event estimation toward the sun.
        if (has_light) {
            let ph = cloud_phase(dot(l, d), 1.0);
            L += thr * ph * ref_sun_radiance(p, l, sun_color) * ref_transmittance(p, l, sigma_t_unit);
        }
        d = ref_scatter_dir(d);
        let s2 = cloud_layer_segment(p, d);
        seg_t0 = max(s2.x, 0.0);
        seg_t1 = select(0.0, min(s2.y, CLOUD_MAX_DISTANCE_M), s2.z > 0.5);
        if (bounce > 4u) {
            let q = clamp(max(thr.x, max(thr.y, thr.z)), 0.05, 1.0);
            if (ref_rand() > q) { break; }
            thr /= q;
        }
    }
    var o: RefOut;
    o.c = vec4(L, tr);
    o.d = select(0.0, dsum / wsum, wsum > 0.0);
    return o;
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

    @if(CLOUD_REFERENCE) {
    if (seg.z > 0.5 && t_end > t_start) {
        let rp = cloud_reference_trace(ray_origin, ray_dir, t_start, t_end, half_px, sample_sky_irradiance(r));
        weighted_depth = rp.d;
        var lum = rp.c.rgb;
        if (rp.c.a < 0.999 && weighted_depth > 0.0) {
            let jr = fract(interleaved_gradient_noise(vec2<f32>(half_px), view.frame_count % 64u) + 0.37);
            let t_ap = sample_transmittance_lut_segment(r, mu, weighted_depth);
            let haze = cloud_shadowed_haze(uv, ray_origin, ray_dir, weighted_depth, r, mu, jr);
            lum = t_ap * rp.c.rgb + (1.0 - rp.c.a) * haze;
        }
        result = vec4(lum * view.exposure, rp.c.a);
    }
    }
    @if(!CLOUD_REFERENCE) {
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
            var ground_rad = sky_irr * CLOUD_GROUND_ALBEDO;

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
                let cos_theta = dot(l, ray_dir);
                // Beer-Powder (HZD15): dark edges only when looking away from the sun, and only on
                // the single-scatter octave (it models missing in-scatter near the surface, so it
                // must not also kill the multi-scatter octaves or sun-facing edges).
                let powder = 1.0 - exp(-2.0 * light_optical_depth);
                let away = saturate(0.5 - 0.5 * cos_theta);
                let dark_edge = mix(1.0, powder, CLOUD_DARK_EDGE_STRENGTH * away);
                // Sunlit ground below this sample (Lambert, albedo): sun irradiance through the
                // atmosphere and the cloud shadow map at the ground point.
                if (light_i == 0u) {
                    let r_g = atmosphere.inner_radius;
                    let p_g = up * r_g;
                    let mu_g = dot(l, up);
                    let e_sun = (*light).color.rgb * sample_transmittance_lut(r_g, mu_g) * max(mu_g, 0.0)
                        * sample_cloud_shadow_map(p_g, l, false);
                    ground_rad = CLOUD_GROUND_ALBEDO * (e_sun / PI + sky_irr);
                }
                var direct = 0.0;
                var a = 1.0;
                var b = 1.0;
                var c = 1.0;
                // Extinction-driven Lambert floor on the phase (RDR2 slide 30).
                // Isotropic (1/4pi) floor: the old 1/pi floor saturated at the current extinction
                // and over-counted back-scatter 4x.
                let phase_floor = (0.25 / PI) * saturate(sigma_t * CLOUD_BACK_SCATTER_EXT_SCALE);
                for (var o = 0u; o < MS_OCTAVES; o++) {
                    let phase = max(cloud_phase(cos_theta, c), phase_floor);
                    direct += a * pow(t_cloud, b) * phase * select(1.0, dark_edge, o == 0u);
                    a *= MS_A;
                    b *= MS_B;
                    c *= MS_C;
                }
                direct_sum += (*light).color.rgb * t_atm * direct;
            }
            let ambient = cloud_ambient_term(sky_irr, ground_rad, h_frac, sigma_t,
                cloud_layer.cloud_layer_end - cloud_layer.cloud_layer_start);
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

    }
    textureStore(out_clouds, id.xy, result);
    textureStore(out_clouds_depth, id.xy, vec4(weighted_depth, 0.0, 0.0, 0.0));
}

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
    bruneton_functions::ray_intersects_ground,
    functions::{
        uv_to_ray_direction, uv_to_ndc, ndc_to_camera_dist, get_view_position,
        sample_transmittance_lut, sample_multiscattering_lut, sample_cloud_shadow_map,
        hg_phase,
    },
    clouds::{
        cloud_layer_segment, get_cloud_medium_density, get_cloud_scattering_coeff,
        get_cloud_absorption_coeff,
    },
};

@group(0) @binding(13) var depth_texture: texture_depth_2d;
@group(0) @binding(26) var out_clouds: texture_storage_2d<rgba16float, write>;
@group(0) @binding(27) var out_clouds_depth: texture_storage_2d<r32float, write>;

// Hard caps for the first version of the pass; these become `VolumetricCloudSettings` (design §3.3).
const CLOUD_MIN_STEPS: f32 = 16.0;
const CLOUD_MAX_STEPS: f32 = 64.0;
// Target step length in meters; long grazing segments clamp to CLOUD_MAX_STEPS.
const CLOUD_TARGET_STEP_M: f32 = 200.0;
// Beyond this distance clouds are fully hidden by aerial perspective anyway.
const CLOUD_MAX_DISTANCE_M: f32 = 150000.0;
const CLOUD_MIN_TRANSMITTANCE: f32 = 0.01;

// Multiple-scattering octaves (Wrenninge 2013; FB16 §5.8 eq. 19-20), with a <= b.
const MS_OCTAVES: u32 = 2u;
const MS_A: f32 = 0.5; // scattering attenuation per octave
const MS_B: f32 = 0.5; // extinction attenuation per octave
const MS_C: f32 = 0.5; // phase eccentricity attenuation per octave
// Sky ambient scale on top of the atmosphere multiple-scattering LUT term.
const CLOUD_AMBIENT_SCALE: f32 = 1.0;

// Order in which the 4 pixels of each 2x2 half-res block are traced (shared with reconstruct).
const CHECKER_OFFSETS: array<vec2<u32>, 4> = array(
    vec2(0u, 0u), vec2(1u, 1u), vec2(1u, 0u), vec2(0u, 1u)
);

fn cloud_phase(cos_theta: f32, g_scale: f32) -> f32 {
    // Dual-lobe HG, same lobes as the previous in-atmosphere integration.
    return mix(hg_phase(cos_theta, 0.85 * g_scale), hg_phase(cos_theta, -0.2 * g_scale), 0.5);
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
        let seg_len = t_end - t_start;
        let steps = clamp(ceil(seg_len / CLOUD_TARGET_STEP_M), CLOUD_MIN_STEPS, CLOUD_MAX_STEPS);
        let dt = seg_len / steps;

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

        for (var i = 0.0; i < steps; i += 1.0) {
            let t = t_start + (i + jitter) * dt;
            let p = ray_origin + ray_dir * t;
            let r_p = length(p);
            let density = get_cloud_medium_density(r_p, p);
            if (density <= 0.0) {
                continue;
            }
            let sigma_t = density * sigma_t_unit;
            let sigma_s = density * sigma_s_unit;
            let up = p / r_p;

            var scattered = vec3(0.0);
            for (var light_i: u32 = 0u; light_i < lights.n_directional_lights; light_i++) {
                let light = &lights.directional_lights[light_i];
                let l = (*light).direction_to_light;
                let mu_light = dot(l, up);
                let t_atm = sample_transmittance_lut(r_p, mu_light)
                    * f32(!ray_intersects_ground(r_p, mu_light));
                let t_cloud = sample_cloud_shadow_map(p, l, false);
                let cos_theta = dot(l, ray_dir);
                var direct = 0.0;
                var a = 1.0;
                var b = 1.0;
                var c = 1.0;
                for (var o = 0u; o < MS_OCTAVES; o++) {
                    direct += a * pow(t_cloud, b) * cloud_phase(cos_theta, c);
                    a *= MS_A;
                    b *= MS_B;
                    c *= MS_C;
                }
                // Isotropic sky ambient approximated by the atmosphere multiple-scattering LUT
                // (replaced by a sky-irradiance reduction in roadmap step 4a).
                let ambient = sample_multiscattering_lut(r_p, mu_light) * CLOUD_AMBIENT_SCALE;
                scattered += (*light).color.rgb * (t_atm * direct + ambient);
            }
            scattered *= sigma_s;

            // Energy-conserving integration (FB16 §5.6.3 eq. 17).
            let step_t = exp(-sigma_t * dt);
            luminance += transmittance * (scattered - scattered * step_t) / max(sigma_t, 1e-9);
            let w = transmittance * (1.0 - step_t);
            depth_sum += w * t;
            depth_weight_sum += w;
            transmittance *= step_t;
            if (transmittance < CLOUD_MIN_TRANSMITTANCE) {
                break;
            }
        }

        if (transmittance < CLOUD_MIN_TRANSMITTANCE) {
            // Renormalize so the opaque cutoff doesn't leave the residual background visible.
            luminance /= max(1.0 - transmittance, 1e-4);
            transmittance = 0.0;
        }
        result = vec4(luminance * view.exposure, transmittance);
        weighted_depth = select(0.0, depth_sum / depth_weight_sum, depth_weight_sum > 0.0);
    }

    textureStore(out_clouds, id.xy, result);
    textureStore(out_clouds_depth, id.xy, vec4(weighted_depth, 0.0, 0.0, 0.0));
}

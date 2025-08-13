#import bevy_pbr::{
    mesh_view_types::{Lights, DirectionalLight},
    atmosphere::{
        types::{Atmosphere, AtmosphereSettings},
        bindings::{atmosphere, settings, view, lights, aerial_view_lut_out},
        functions::{
            sample_transmittance_lut, sample_atmosphere, rayleigh, henyey_greenstein,
            sample_multiscattering_lut, AtmosphereSample, sample_local_inscattering,
            get_local_r, get_local_up, view_radius, uv_to_ndc, max_atmosphere_distance,
            uv_to_ray_direction, MIDPOINT_RATIO
        },
    }
}


@group(0) @binding(13) var aerial_view_lut_out: texture_storage_3d<rgba16float, write>;

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) idx: vec3<u32>) {
    if any(idx.xy > settings.aerial_view_lut_size.xy) { return; }

    let uv = (vec2<f32>(idx.xy) + 0.5) / vec2<f32>(settings.aerial_view_lut_size.xy);
    let ray_dir = uv_to_ray_direction(uv);
    let r = view_radius();
    let mu = ray_dir.y;
    let t_max = settings.aerial_view_lut_max_distance;

    var prev_t = 0.0;
    var total_inscattering = vec3(0.0);
    var throughput = vec3(1.0);

    for (var slice_i: u32 = 0; slice_i < settings.aerial_view_lut_size.z; slice_i++) {
        for (var step_i: u32 = 0; step_i < settings.aerial_view_lut_samples; step_i++) {
            let t_i = t_max * (f32(slice_i) + ((f32(step_i) + MIDPOINT_RATIO) / f32(settings.aerial_view_lut_samples))) / f32(settings.aerial_view_lut_size.z);
            let dt = (t_i - prev_t);
            prev_t = t_i;

            let local_r = get_local_r(r, mu, t_i);
            let local_up = get_local_up(r, t_i, ray_dir.xyz);

            let local_atmosphere = sample_atmosphere(local_r);
            let sample_optical_depth = local_atmosphere.extinction * dt;
            let sample_transmittance = exp(-sample_optical_depth);

            // evaluate one segment of the integral
            var inscattering = sample_local_inscattering(local_atmosphere, ray_dir.xyz, local_r, local_up);

            // Analytical integration of the single scattering term in the radiance transfer equation
            let s_int = (inscattering - inscattering * sample_transmittance) / local_atmosphere.extinction;
            total_inscattering += throughput * s_int;

            throughput *= sample_transmittance;
            if all(throughput < vec3(0.001)) {
                break;
            }
        }

        // Store in log space to allow linear interpolation of exponential values between slices
        let log_inscattering = log(max(total_inscattering, vec3(1e-6)));
        textureStore(aerial_view_lut_out, vec3(vec2<u32>(idx.xy), slice_i), vec4(log_inscattering, 0.0));
    }
}

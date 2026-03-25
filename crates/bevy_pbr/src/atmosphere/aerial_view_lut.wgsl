#import bevy_pbr::{
    atmosphere::{
        bindings::settings,
        functions::{
            sample_density_lut, sample_local_inscattering, uv_to_ray_direction, get_view_position,
            MIDPOINT_RATIO, MIN_EXTINCTION, ABSORPTION_DENSITY, SCATTERING_DENSITY,
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
    let world_pos = get_view_position();

    let r = length(world_pos);
    let t_max = settings.aerial_view_lut_max_distance;

    var prev_t = 0.0;
    var total_inscattering = vec3(0.0);
    var throughput = vec3(1.0);

    for (var slice_i: u32 = 0; slice_i < settings.aerial_view_lut_size.z; slice_i++) {
        for (var step_i: u32 = 0; step_i < settings.aerial_view_lut_samples; step_i++) {
            let t_i = t_max * (f32(slice_i) + ((f32(step_i) + MIDPOINT_RATIO) / f32(settings.aerial_view_lut_samples))) / f32(settings.aerial_view_lut_size.z);
            let dt = (t_i - prev_t);
            prev_t = t_i;

            let sample_pos = world_pos + ray_dir * t_i;
            let local_r = length(sample_pos);
            let local_up = normalize(sample_pos);

            let absorption = sample_density_lut(local_r, ABSORPTION_DENSITY);
            let scattering = sample_density_lut(local_r, SCATTERING_DENSITY);
            let extinction = absorption + scattering;

            let sample_optical_depth = extinction * dt;
            let sample_transmittance = exp(-sample_optical_depth);

            // evaluate one segment of the integral
            var inscattering = sample_local_inscattering(scattering, ray_dir, sample_pos);

            // Analytical integration of the single scattering term in the radiance transfer equation
            let s_int = (inscattering - inscattering * sample_transmittance) / max(extinction, MIN_EXTINCTION);
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

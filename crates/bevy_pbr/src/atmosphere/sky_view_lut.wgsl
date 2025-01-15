#import bevy_pbr::{
    mesh_view_types::Lights,
    atmosphere::{
        types::{Atmosphere, AtmosphereSettings},
        bindings::{atmosphere, view, settings},
        functions::{
            sample_atmosphere, get_local_up, AtmosphereSample,
            sample_local_inscattering, get_local_r, view_radius,
            direction_view_to_world, max_atmosphere_distance, 
            direction_atmosphere_to_world, sky_view_lut_uv_to_zenith_azimuth,
        },
    }
}

#import bevy_render::{
    view::View,
    maths::HALF_PI,
}
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(13) var sky_view_lut_out: texture_storage_2d<rgba16float, write>;

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) idx: vec3<u32>) {
    let uv = vec2<f32>(idx.xy) / vec2<f32>(settings.sky_view_lut_size);

    let r = view_radius();
    var zenith_azimuth = sky_view_lut_uv_to_zenith_azimuth(r, uv);

    let ray_dir_as = zenith_azimuth_to_ray_dir(zenith_azimuth.x, zenith_azimuth.y);
    let ray_dir_ws = direction_atmosphere_to_world(ray_dir_as);

    let mu = ray_dir_ws.y;
    let t_max = max_atmosphere_distance(r, mu);

    // Raymarch with quadratic distribution
    let sample_count = mix(1.0, f32(settings.sky_view_lut_samples), clamp(t_max * 0.01, 0.0, 1.0));
    let sample_count_floor = floor(sample_count);
    let t_max_floor = t_max * sample_count_floor / sample_count;
    var total_inscattering = vec3(0.0);
    var optical_depth = vec3(0.0);
    var throughput = vec3(1.0);
    for (var s = 0.0; s < sample_count; s += 1.0) {
        // Use quadratic distribution like reference
        var t0 = (s / sample_count_floor);
        var t1 = ((s + 1.0) / sample_count_floor);
        t0 = t0 * t0;
        t1 = t1 * t1;
        t1 = select(t_max_floor * t1, t_max, t1 > 1.0);
        let t_i = t_max_floor * t0 + (t1 - t_max_floor * t0) * 0.3;
        let dt_i = t1 - t_max_floor * t0;

        let local_r = get_local_r(r, mu, t_i);
        let local_up = get_local_up(r, t_i, ray_dir_ws);
        let local_atmosphere = sample_atmosphere(local_r);

        let sample_optical_depth = local_atmosphere.extinction * dt_i;
        let sample_transmittance = exp(-sample_optical_depth);
        optical_depth += sample_optical_depth;

        // We set the transmittance_to_sample to 1.0 since we're using the analytical integration
        let inscattering = sample_local_inscattering(
            local_atmosphere,
            vec3(1.0),
            ray_dir_ws,
            local_r,
            local_up
        );

        // Analytical integration of the single scattering term in the radiance transfer equation
        let s_int = (inscattering - inscattering * sample_transmittance) / local_atmosphere.extinction;
        total_inscattering += throughput * s_int;
        
        throughput *= sample_transmittance;
        if all(throughput < vec3(0.001)) {
            break;
        }
    }

    textureStore(sky_view_lut_out, idx.xy, vec4(total_inscattering, 1.0));
}

fn zenith_azimuth_to_ray_dir(zenith: f32, azimuth: f32) -> vec3<f32> {
    let sin_zenith = sin(zenith);
    let mu = cos(zenith);
    let sin_azimuth = sin(azimuth);
    let cos_azimuth = cos(azimuth);
    return vec3(sin_azimuth * sin_zenith, mu, -cos_azimuth * sin_zenith);
}

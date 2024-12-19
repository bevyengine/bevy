#import bevy_pbr::{
    fast_math::fast_acos,
    mesh_view_types::Lights,
    atmosphere::{
        types::{Atmosphere, AtmosphereSettings},
        bindings::{atmosphere, view, settings},
        functions::{
            sample_atmosphere, get_local_up, AtmosphereSample,
            sample_local_inscattering, get_local_r, view_radius,
            direction_view_to_world, max_atmosphere_distance, 
            direction_atmosphere_to_world, sky_view_lut_uv_to_lat_long,
            get_horizon_zenith, zenith_to_altitude
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
    let uv = (vec2<f32>(idx.xy) + vec2(0.5)) / vec2<f32>(settings.sky_view_lut_size);

    let r = view_radius();
    var lat_long = sky_view_lut_uv_to_lat_long(uv);
    lat_long.x += zenith_to_altitude(get_horizon_zenith(r));

    let ray_dir_as = lat_long_to_ray_dir_as(lat_long.x, lat_long.y);
    let ray_dir = direction_atmosphere_to_world(ray_dir_as);

    let mu = ray_dir.y;

    let t_max = max_atmosphere_distance(r, mu);
    let dt = t_max / f32(settings.sky_view_lut_samples);

    var total_inscattering = vec3(0.0);
    var optical_depth = vec3(0.0);
    for (var step_i: u32 = 0u; step_i < settings.sky_view_lut_samples; step_i++) {
        let t_i = dt * (f32(step_i) + 0.3);
        let local_r = get_local_r(r, mu, t_i);
        let local_up = get_local_up(r, t_i, ray_dir);

        let local_atmosphere = sample_atmosphere(local_r);
        optical_depth += local_atmosphere.extinction * dt;
        let transmittance_to_sample = exp(-optical_depth);

        var local_inscattering = sample_local_inscattering(local_atmosphere, transmittance_to_sample, ray_dir, local_r, local_up);
        total_inscattering += local_inscattering * dt;
    }

    textureStore(sky_view_lut_out, idx.xy, vec4(total_inscattering, 1.0));
}

fn lat_long_to_ray_dir_as(lat: f32, long: f32) -> vec3<f32> {
    let sin_lat = sin(lat);
    let sin_long = sin(long);
    let cos_long = cos(long);
    return normalize(vec3(sin_long, sin_lat, -cos_long));
}

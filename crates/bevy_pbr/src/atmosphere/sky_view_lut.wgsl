#import bevy_pbr::{
    mesh_view_types::Lights,
    atmosphere::{
        types::{Atmosphere, AtmosphereSettings},
        bindings::{atmosphere, view, settings},
        functions::{
            sample_atmosphere, get_local_up,
            sample_transmittance_lut, sample_multiscattering_lut, rayleigh, henyey_greenstein,
            distance_to_bottom_atmosphere_boundary, ray_intersects_ground, AtmosphereSample,
            sky_view_lut_uv_to_lat_long, sample_local_inscattering, get_local_r, 
        },
        bruneton_functions::{distance_to_top_atmosphere_boundary, distance_to_bottom_atmosphere_boundary, ray_intersects_ground}
    }
}

#import bevy_render::view::View;
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@fragment
fn main(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let lat_long = sky_view_lut_uv_to_lat_long(in.uv);
    let view_dir = get_ray_direction(lat_long);
    let r = view.world_position.y / 1000.0 + atmosphere.bottom_radius; //we center the sky view on the planet ground
    let mu = view_dir.y;

    let atmosphere_dist = distance_to_top_atmosphere_boundary(r, mu);
    let step_length = atmosphere_dist / f32(settings.sky_view_lut_samples);

    var total_inscattering = vec3(0.0);
    var optical_depth = vec3(0.0);
    for (var step_i: u32 = 0u; step_i < settings.sky_view_lut_samples; step_i++) {
        let displacement = step_length * (f32(step_i) + 0.5); //todo: 0.3???;
        let local_r = get_local_r(r, mu, displacement);
        let local_up = get_local_up(r, displacement * view_dir);

        let local_atmosphere = sample_atmosphere(local_r);
        optical_depth += local_atmosphere.extinction * step_length; //TODO: Units between atmosphere and step_length
        let transmittance_to_sample = exp(-optical_depth);

        var local_inscattering = sample_local_inscattering(local_atmosphere, transmittance_to_sample, view_dir, local_r, local_up);
        total_inscattering += local_inscattering * step_length;
    }

    return vec4(total_inscattering, 1.0);
}

//lat-long projection [-pi, pi] x [-pi/2, pi/2] -> S^2
fn get_ray_direction(lat_long: vec2<f32>) -> vec3<f32> {
    let cos_long = cos(lat_long.y);
    let sin_long = sin(lat_long.y);
    let horizontal_rotation = mat2x2(cos_long, -sin_long, sin_long, cos_long);
    let horizontal = horizontal_rotation * vec2(-view.world_from_view[2].xz);

    return normalize(vec3(horizontal.x, sin(lat_long.x), horizontal.y));
}

#import bevy_pbr::{
    mesh_view_types::Lights,
    atmosphere::{
        types::{Atmosphere, AtmosphereSettings},
        functions::{
            distance_to_top_atmosphere_boundary, sample_atmosphere, 
            sample_transmittance_lut, sample_multiscattering_lut, rayleigh, henyey_greenstein,
            distance_to_bottom_atmosphere_boundary, ray_intersects_ground, AtmosphereSample,
            sky_view_lut_uv_to_lat_long,
        }
    }
}
#import bevy_render::view::View;
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var<uniform> atmosphere: Atmosphere;
@group(0) @binding(1) var<uniform> settings: AtmosphereSettings;
@group(0) @binding(2) var<uniform> view: View;
@group(0) @binding(3) var<uniform> lights: Lights;
@group(0) @binding(4) var transmittance_lut: texture_2d<f32>;
@group(0) @binding(5) var transmittance_lut_sampler: sampler;
@group(0) @binding(6) var multiscattering_lut: texture_2d<f32>;
@group(0) @binding(7) var multiscattering_lut_sampler: sampler;

@fragment
fn main(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let lat_long = sky_view_lut_uv_to_lat_long(in.uv);
    let view_dir = get_ray_direction(lat_long);
    let view_pos = vec3(view.world_position.x, 0, view.world_position.z);

    let atmosphere_dist = distance_to_top_atmosphere_boundary(atmosphere, view_pos.y, view_dir.y);
    let step_length = atmosphere_dist / f32(settings.sky_view_lut_samples) / 1000.0;

    var total_inscattering = vec3(0.0);
    var optical_depth = vec3(0.0);
    for (var step_i: u32 = 0u; step_i < settings.sky_view_lut_samples; step_i++) {
        let pos = view_pos + step_length * view_dir;
        let altitude = pos.y;

        let local_atmosphere = sample_atmosphere(atmosphere, altitude);
        optical_depth += local_atmosphere.extinction * step_length; //TODO: Units between atmosphere and step_length
        let transmittance_to_sample = exp(-optical_depth);

        var local_inscattering = sample_local_inscattering(local_atmosphere, transmittance_to_sample, view_dir, altitude);
        total_inscattering += local_inscattering * step_length;
    }

    return vec4(total_inscattering, 1.0);
}

fn sample_local_inscattering(local_atmosphere: AtmosphereSample, transmittance_to_sample: vec3<f32>, view_dir: vec3<f32>, altitude: f32) -> vec3<f32> {
    //TODO: storing these outside the loop saves several multiplications, but at the cost of an extra vector register
    var rayleigh_scattering = vec3(0.0);
    var mie_scattering = vec3(0.0);
    for (var light_i: u32 = 0u; light_i < lights.n_directional_lights; light_i++) {
        let light = &lights.directional_lights[light_i];
        let light_cos_azimuth = (*light).direction_to_light.y;
        let neg_LdotV = dot(view_dir, (*light).direction_to_light);
        let rayleigh_phase = rayleigh(neg_LdotV);
        let mie_phase = henyey_greenstein(neg_LdotV, atmosphere.mie_asymmetry);

        let transmittance_to_light = sample_transmittance_lut(atmosphere, transmittance_lut, transmittance_lut_sampler, altitude, light_cos_azimuth);
        let shadow_factor = transmittance_to_light * f32(!ray_intersects_ground(atmosphere, altitude, light_cos_azimuth));

        let psi_ms = sample_multiscattering_lut(atmosphere, multiscattering_lut, multiscattering_lut_sampler, altitude, light_cos_azimuth);

        //rayleigh_scattering += (transmittance_to_sample * rayleigh_phase + psi_ms) * (*light).color.rgb; //TODO: what is color.a?
        //mie_scattering += (transmittance_to_sample * mie_phase + psi_ms) * (*light).color.rgb;

        rayleigh_scattering += (transmittance_to_sample * shadow_factor * rayleigh_phase + psi_ms) * (*light).color.rgb; //TODO: what is color.a?
        mie_scattering += (transmittance_to_sample * shadow_factor * mie_phase + psi_ms) * (*light).color.rgb;
    }
    return local_atmosphere.rayleigh_scattering * rayleigh_scattering + local_atmosphere.mie_scattering * mie_scattering;
}

//lat-long projection [0,1] x [0,1] --> [-pi, pi] x [-pi/2, pi/2]
fn get_ray_direction(lat_long: vec2<f32>) -> vec3<f32> {
    let cos_long = cos(lat_long.y);
    let sin_long = sin(lat_long.y);
    let horizontal_rotation = mat2x2(cos_long, -sin_long, sin_long, cos_long);
    let horizontal = horizontal_rotation * vec2(-view.world_from_view[2].xz);

    return normalize(vec3(horizontal.x, sin(lat_long.x), horizontal.y));
}

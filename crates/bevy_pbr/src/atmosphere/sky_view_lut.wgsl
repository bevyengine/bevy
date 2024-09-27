#import bevy_pbr::{
    atmosphere::{
        types::{Atmosphere, AtmosphereSettings},
        functions::{
            distance_to_top_atmosphere_boundary, sample_atmosphere, 
            sample_transmittance_lut, sample_multiscattering_lut, rayleigh, henyey_greenstein,
            distance_to_bottom_atmosphere_boundary,
        }
    }
    mesh_view_types::Lights
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

fn magically_get_view_direction(in: FullscreenVertexOutput) -> vec3<f32> { //TODO: HOW
    return vec3(0.0);
}

fn magically_get_view_position() -> vec3<f32> {
    return vec3(0.0);
}

@fragment
fn main(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let view_dir = magically_get_view_direction(in);
    let view_pos = magically_get_view_position();

    let atmosphere_dist = distance_to_top_atmosphere_boundary(atmosphere, view_pos.y, view_dir.y);
    let step_length = atmosphere_dist / f32(settings.sky_view_lut_samples);

    var inscattered_illuminance = vec3(0.0);
    var optical_depth = vec3(0.0);
    for (var step_i: u32 = 0u; step_i < settings.sky_view_lut_samples; step_i++) {
        let pos = view_pos + step_length * view_dir;
        let view_height = pos.y;

        let local_atmosphere = sample_atmosphere(atmosphere, view_height);
        optical_depth += local_atmosphere.extinction * step_length;
        let transmittance_to_sample = exp(-optical_depth);

        /*let local_illuminance = sample_local_inscattering(
            atmosphere, &lights, transmittance_lut, transmittance_lut_sampler,
            multiscattering_lut, multiscattering_lut_sampler,
            transmittance_to_sample, r, view_dir
        );*/

        var local_illuminance = vec3(0.0);
        for (var light_i: u32 = 0u; light_i < lights.n_directional_lights; light_i++) {
            let light = &lights.directional_lights[light_i];
            let mu_light = (*light).direction_to_light.y;
            let neg_LdotV = dot(view_dir, (*light).direction_to_light);
            let rayleigh_phase = rayleigh(neg_LdotV);
            let mie_phase = henyey_greenstein(neg_LdotV, atmosphere.mie_asymmetry);
            let phase = rayleigh_phase + mie_phase; //TODO: check this

            let ground_dist = distance_to_bottom_atmosphere_boundary(atmosphere, view_height, mu_light);
            let atmosphere_dist = distance_to_top_atmosphere_boundary(atmosphere, view_height, mu_light);
            let vis = step(atmosphere_dist, ground_dist); //TODO: need to check that the intersection tests return infinity on a miss
            let transmittance_to_light = sample_transmittance_lut(atmosphere, transmittance_lut, transmittance_lut_sampler, view_height, mu_light);
            let shadow_factor = transmittance_to_light * vis;

            let psi_ms = sample_multiscattering_lut(atmosphere, multiscattering_lut, multiscattering_lut_sampler, view_height, mu_light);

            local_illuminance += (transmittance_to_sample * shadow_factor * phase + psi_ms) * (*light).color.rgb; //TODO: what is color.a?
        }

        inscattered_illuminance += local_atmosphere.scattering * local_illuminance * step_length;
    }

    return vec4(inscattered_illuminance, 1.0);
}
